//! Timers and tweens for scripts: `after(seconds, fn)` and `every(seconds,
//! fn)` call back later, `tween(entity, {x = 100}, seconds)` moves numeric
//! attributes over time, and `cancel(id)` stops either. They run on game
//! time, at the start of each frame before the scripts' `update` hooks, and
//! belong to the entity whose script made them: they stop when it's removed,
//! when the scene changes and when the game stops.

use super::{LuaScripting, parse_uuid};
use crate::logger::LOGGER;
use crate::physics::PhysicsEngine;
use crate::scene::{AttributeValue, SceneManager};
use mlua::{Function as LuaFunction, Table, Value};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;
use uuid::Uuid;

/// Most timers and tweens waiting at once, so a script making them every
/// frame without end fails loudly instead of slowing the game to a crawl.
pub const MAX_TIMERS: usize = 10_000;

/// How a tween eases from its start to its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    BackIn,
    BackOut,
    ElasticOut,
    BounceOut,
}

/// Every ease, with the name scripts use for it.
pub const EASES: [(Ease, &str); 14] = [
    (Ease::Linear, "linear"),
    (Ease::QuadIn, "quad_in"),
    (Ease::QuadOut, "quad_out"),
    (Ease::QuadInOut, "quad_in_out"),
    (Ease::CubicIn, "cubic_in"),
    (Ease::CubicOut, "cubic_out"),
    (Ease::CubicInOut, "cubic_in_out"),
    (Ease::SineIn, "sine_in"),
    (Ease::SineOut, "sine_out"),
    (Ease::SineInOut, "sine_in_out"),
    (Ease::BackIn, "back_in"),
    (Ease::BackOut, "back_out"),
    (Ease::ElasticOut, "elastic_out"),
    (Ease::BounceOut, "bounce_out"),
];

impl Ease {
    /// The ease named `name`, as listed in [`EASES`].
    pub fn parse(name: &str) -> Option<Self> {
        EASES
            .iter()
            .find(|(_, candidate)| *candidate == name)
            .map(|(ease, _)| *ease)
    }

    /// How far along the tween is, from 0 to 1, `t` into its time (also 0
    /// to 1). Back and elastic eases overshoot 1 on the way.
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        // easings.net's formulas
        const BACK: f64 = 1.70158;
        match self {
            Ease::Linear => t,
            Ease::QuadIn => t * t,
            Ease::QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::QuadInOut if t < 0.5 => 2.0 * t * t,
            Ease::QuadInOut => 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0,
            Ease::CubicIn => t * t * t,
            Ease::CubicOut => 1.0 - (1.0 - t).powi(3),
            Ease::CubicInOut if t < 0.5 => 4.0 * t * t * t,
            Ease::CubicInOut => 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0,
            Ease::SineIn => 1.0 - (t * PI / 2.0).cos(),
            Ease::SineOut => (t * PI / 2.0).sin(),
            Ease::SineInOut => -((PI * t).cos() - 1.0) / 2.0,
            Ease::BackIn => (BACK + 1.0) * t * t * t - BACK * t * t,
            Ease::BackOut => {
                let u = t - 1.0;
                1.0 + (BACK + 1.0) * u * u * u + BACK * u * u
            }
            Ease::ElasticOut if t == 0.0 || t == 1.0 => t,
            Ease::ElasticOut => {
                2f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * (2.0 * PI / 3.0)).sin() + 1.0
            }
            Ease::BounceOut => bounce_out(t),
        }
    }
}

fn bounce_out(t: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let t = t - 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        let t = t - 2.25 / D;
        N * t * t + 0.9375
    } else {
        let t = t - 2.625 / D;
        N * t * t + 0.984375
    }
}

/// What a tween does when it reaches its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TweenLoop {
    /// Stops there, and calls its `on_done`
    Once,
    /// Starts over from the start
    Restart,
    /// Goes back to the start, then to the end again, and so on
    PingPong,
}

struct Timer {
    id: i64,
    owner: Uuid,
    due: f64,
    /// `every`'s interval; `None` for `after`
    interval: Option<f64>,
    callback: LuaFunction,
}

struct Tween {
    id: i64,
    owner: Uuid,
    entity: Uuid,
    /// Attribute names and the values they go to, and where they started
    /// once the tween started
    targets: Vec<(String, f64)>,
    starts: Option<Vec<f64>>,
    /// When it starts (after its delay), and for how long
    start_time: f64,
    duration: f64,
    ease: Ease,
    looping: TweenLoop,
    on_done: Option<LuaFunction>,
}

/// The timers and tweens waiting, in the order they were made.
#[derive(Default)]
pub(crate) struct Scheduler {
    next_id: i64,
    timers: Vec<Timer>,
    tweens: Vec<Tween>,
}

impl Scheduler {
    fn next_id(&mut self) -> Result<i64, mlua::Error> {
        if self.timers.len() + self.tweens.len() >= MAX_TIMERS {
            return Err(mlua::Error::external(format!(
                "more than {} timers and tweens are waiting: are they made every frame?",
                MAX_TIMERS
            )));
        }
        self.next_id += 1;
        Ok(self.next_id)
    }

    /// Stop a timer or tween; returns whether there was one.
    fn cancel(&mut self, id: i64) -> bool {
        let before = self.timers.len() + self.tweens.len();
        self.timers.retain(|timer| timer.id != id);
        self.tweens.retain(|tween| tween.id != id);
        self.timers.len() + self.tweens.len() < before
    }

    /// How many timers and tweens are waiting.
    pub(crate) fn len(&self) -> usize {
        self.timers.len() + self.tweens.len()
    }
}

/// A tween's position, from 0 to 1, `elapsed` seconds into it, and whether
/// it's over (only for tweens that play once).
fn progress(elapsed: f64, duration: f64, looping: TweenLoop) -> (f64, bool) {
    if duration <= 0.0 {
        return (1.0, looping == TweenLoop::Once);
    }
    let cycles = elapsed / duration;
    match looping {
        TweenLoop::Once => (cycles.min(1.0), cycles >= 1.0),
        TweenLoop::Restart => (cycles.fract(), false),
        TweenLoop::PingPong => {
            let phase = cycles % 2.0;
            (if phase <= 1.0 { phase } else { 2.0 - phase }, false)
        }
    }
}

impl LuaScripting {
    /// Fire the timers whose time came and move the tweens on, for the frame
    /// that `accumulated_time` reached. Timers and tweens of entities gone
    /// from the scene are dropped first.
    pub(super) fn run_timers(&mut self, scene_id: Uuid) {
        let (Some(scene_manager), Some(physics)) =
            (self.scene_manager.clone(), self.physics_engine.clone())
        else {
            return;
        };
        let now = self.accumulated_time;
        {
            let manager = scene_manager.borrow();
            let exists = |id: &Uuid| {
                manager
                    .get_scene(scene_id)
                    .is_some_and(|scene| scene.entities.contains_key(id))
            };
            let mut scheduler = self.scheduler.borrow_mut();
            scheduler.timers.retain(|timer| exists(&timer.owner));
            scheduler
                .tweens
                .retain(|tween| exists(&tween.owner) && exists(&tween.entity));
        }
        self.fire_timers(now);
        self.move_tweens(scene_id, now, &scene_manager, &physics);
    }

    fn fire_timers(&mut self, now: f64) {
        // The ones due, earliest first; timers made by their callbacks wait
        // for the next frame
        let mut due: Vec<(f64, i64)> = self
            .scheduler
            .borrow()
            .timers
            .iter()
            .filter(|timer| timer.due <= now)
            .map(|timer| (timer.due, timer.id))
            .collect();
        due.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, id) in due {
            // Rescheduled before the call, so the callback can cancel it
            let (owner, callback, repeats) = {
                let mut scheduler = self.scheduler.borrow_mut();
                let Some(index) = scheduler.timers.iter().position(|timer| timer.id == id) else {
                    continue; // cancelled by an earlier callback
                };
                let timer = &mut scheduler.timers[index];
                let (owner, callback) = (timer.owner, timer.callback.clone());
                match timer.interval {
                    Some(interval) => {
                        // At most once a frame: a shorter interval skips calls
                        timer.due += interval;
                        if timer.due <= now {
                            timer.due = now + interval;
                        }
                    }
                    None => {
                        scheduler.timers.remove(index);
                    }
                }
                let repeats = scheduler.timers.iter().any(|timer| timer.id == id);
                (owner, callback, repeats)
            };
            let result = self.call_as(owner, || callback.call::<Value>(()));
            let stop = match result {
                Ok(Value::Boolean(false)) => true,
                Ok(_) => false,
                Err(e) => {
                    LOGGER.error(format!(
                        "Timer callback error for entity {} (the timer is stopped): {}",
                        owner, e
                    ));
                    true
                }
            };
            if stop && repeats {
                self.scheduler.borrow_mut().cancel(id);
            }
        }
    }

    fn move_tweens(
        &mut self,
        scene_id: Uuid,
        now: f64,
        scene_manager: &Rc<RefCell<SceneManager>>,
        physics: &Rc<RefCell<PhysicsEngine>>,
    ) {
        let mut finished = Vec::new();
        {
            let mut scheduler = self.scheduler.borrow_mut();
            let mut manager = scene_manager.borrow_mut();
            let Some(scene) = manager.get_scene_mut(scene_id) else {
                return;
            };
            for tween in &mut scheduler.tweens {
                if now < tween.start_time {
                    continue;
                }
                let Ok(entity) = scene.get_entity_mut(tween.entity) else {
                    continue;
                };
                // Where it starts: the values when it starts, not when it was made
                let starts = tween.starts.get_or_insert_with(|| {
                    tween
                        .targets
                        .iter()
                        .map(|(name, end)| {
                            number(entity.attribute(name).map(|a| &a.value)).unwrap_or(*end)
                        })
                        .collect()
                });
                let (t, done) = progress(now - tween.start_time, tween.duration, tween.looping);
                let eased = tween.ease.apply(t);
                let mut moved = false;
                for ((name, end), start) in tween.targets.iter().zip(starts.iter()) {
                    let value = if done {
                        *end
                    } else {
                        start + (end - start) * eased
                    };
                    let Some(attribute) = entity.attribute(name) else {
                        continue;
                    };
                    let new_value = match attribute.value {
                        AttributeValue::Float(_) => AttributeValue::Float(value as f32),
                        AttributeValue::Integer(_) => AttributeValue::Integer(value.round() as i32),
                        _ => continue,
                    };
                    let id = attribute.id;
                    if entity
                        .modify_attribute(id, None, None, Some(new_value))
                        .is_ok()
                    {
                        moved |= matches!(name.as_str(), "x" | "y");
                    }
                }
                if moved {
                    let (x, y) = (entity.get_x(), entity.get_y());
                    physics.borrow_mut().set_position(tween.entity, x, y);
                }
                if done {
                    finished.push(tween.id);
                }
            }
        }
        // Finished tweens go, then their on_done runs (it may tween again)
        for id in finished {
            let removed = {
                let mut scheduler = self.scheduler.borrow_mut();
                let index = scheduler.tweens.iter().position(|tween| tween.id == id);
                index.map(|index| scheduler.tweens.remove(index))
            };
            if let Some(Tween {
                owner,
                on_done: Some(on_done),
                ..
            }) = removed
            {
                let result = self.call_as(owner, || on_done.call::<()>(()));
                if let Err(e) = result {
                    LOGGER.error(format!("Tween on_done error for entity {}: {}", owner, e));
                }
            }
        }
    }

    /// Call into Lua as `entity`'s script would: what it makes is its own.
    /// Runs under the time limit, unless a call is under way already (the
    /// callback then shares its time).
    pub(super) fn call_as<T>(
        &self,
        entity: Uuid,
        call: impl FnOnce() -> mlua::Result<T>,
    ) -> mlua::Result<T> {
        let previous = self.current_entity.replace(Some(entity));
        let result = if self.deadline.get().is_some() {
            call()
        } else {
            self.with_time_limit(call)
        };
        self.current_entity.set(previous);
        result
    }

    pub(crate) fn register_timer_bindings(&mut self) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // after(seconds, fn) -> id: call fn once, `seconds` of game time
        // from now
        let scheduler = Rc::clone(&self.scheduler);
        let current = Rc::clone(&self.current_entity);
        let time = Rc::clone(&self.clock);
        let after =
            self.lua
                .create_function(move |_, (seconds, callback): (f64, LuaFunction)| {
                    let owner = owner("after", &current)?;
                    let seconds = duration("after", seconds, false)?;
                    let mut scheduler = scheduler.borrow_mut();
                    let id = scheduler.next_id()?;
                    scheduler.timers.push(Timer {
                        id,
                        owner,
                        due: time.get() + seconds,
                        interval: None,
                        callback,
                    });
                    Ok(id)
                })?;
        globals.set("after", after)?;

        // every(seconds, fn) -> id: call fn every `seconds` of game time
        // (at most once a frame) until it returns false or is cancelled
        let scheduler = Rc::clone(&self.scheduler);
        let current = Rc::clone(&self.current_entity);
        let time = Rc::clone(&self.clock);
        let every =
            self.lua
                .create_function(move |_, (seconds, callback): (f64, LuaFunction)| {
                    let owner = owner("every", &current)?;
                    let seconds = duration("every", seconds, true)?;
                    let mut scheduler = scheduler.borrow_mut();
                    let id = scheduler.next_id()?;
                    scheduler.timers.push(Timer {
                        id,
                        owner,
                        due: time.get() + seconds,
                        interval: Some(seconds),
                        callback,
                    });
                    Ok(id)
                })?;
        globals.set("every", every)?;

        // cancel(id) -> whether a timer or tween was stopped
        let scheduler = Rc::clone(&self.scheduler);
        let cancel = self
            .lua
            .create_function(move |_, id: i64| Ok(scheduler.borrow_mut().cancel(id)))?;
        globals.set("cancel", cancel)?;

        // tween(entity_id, {attribute = value, ...}, seconds [, options]) ->
        // id: move numeric attributes of an entity of the active scene to
        // these values; options: ease, delay, loop ("restart",
        // "ping_pong"), on_done
        let scheduler = Rc::clone(&self.scheduler);
        let current = Rc::clone(&self.current_entity);
        let time = Rc::clone(&self.clock);
        let manager = self
            .scene_manager
            .clone()
            .ok_or_else(|| mlua::Error::external("Lua session not started"))?;
        let tween = self.lua.create_function(
            move |_, (entity_id, values, seconds, options): (String, Table, f64, Option<Table>)| {
                let owner = owner("tween", &current)?;
                let entity = parse_uuid(&entity_id, "entity")?;
                let seconds = duration("tween", seconds, false)?;
                // Read everything from Lua before borrowing the scene
                let mut targets = Vec::new();
                for pair in values.pairs::<String, Value>() {
                    let (name, value) = pair?;
                    let target = match value {
                        Value::Number(n) => n,
                        Value::Integer(n) => n as f64,
                        _ => return Err(tween_error(format!("'{}' isn't given a number", name))),
                    };
                    if !target.is_finite() {
                        return Err(tween_error(format!(
                            "'{}' isn't given a finite number",
                            name
                        )));
                    }
                    targets.push((name, target));
                }
                if targets.is_empty() {
                    return Err(tween_error("no attributes to move".to_string()));
                }
                targets.sort_by(|a, b| a.0.cmp(&b.0));
                let (ease, delay, looping, on_done) = tween_options(options)?;
                {
                    let manager = manager.borrow();
                    let scene = manager
                        .get_active_scene()
                        .ok_or_else(|| tween_error("no scene is playing".to_string()))?;
                    let entity = scene.get_entity(entity).map_err(|_| {
                        tween_error(format!("entity {} isn't in the playing scene", entity))
                    })?;
                    for (name, _) in &targets {
                        match entity.attribute(name).map(|attribute| &attribute.value) {
                            Some(AttributeValue::Float(_) | AttributeValue::Integer(_)) => {}
                            Some(_) => {
                                return Err(tween_error(format!(
                                    "attribute '{}' isn't a number",
                                    name
                                )));
                            }
                            None => {
                                return Err(tween_error(format!(
                                    "the entity has no attribute '{}'",
                                    name
                                )));
                            }
                        }
                    }
                }
                let mut scheduler = scheduler.borrow_mut();
                let id = scheduler.next_id()?;
                scheduler.tweens.push(Tween {
                    id,
                    owner,
                    entity,
                    targets,
                    starts: None,
                    start_time: time.get() + delay,
                    duration: seconds,
                    ease,
                    looping,
                    on_done,
                });
                Ok(id)
            },
        )?;
        globals.set("tween", tween)?;
        Ok(())
    }
}

fn tween_error(why: String) -> mlua::Error {
    mlua::Error::external(format!("tween: {}", why))
}

/// The entity whose script is running, which owns what it makes.
fn owner(function: &str, current: &Cell<Option<Uuid>>) -> Result<Uuid, mlua::Error> {
    current.get().ok_or_else(|| {
        mlua::Error::external(format!(
            "{}: call it from a script's hooks (init, update, ...), not its top level",
            function
        ))
    })
}

/// A number of seconds from scripts: finite, at least 0 (above 0 when
/// `positive`).
fn duration(function: &str, seconds: f64, positive: bool) -> Result<f64, mlua::Error> {
    let valid = seconds.is_finite()
        && if positive {
            seconds > 0.0
        } else {
            seconds >= 0.0
        };
    if valid {
        Ok(seconds)
    } else {
        let bound = if positive { "above 0" } else { "at least 0" };
        Err(mlua::Error::external(format!(
            "{}: seconds must be {}, not {}",
            function, bound, seconds
        )))
    }
}

type TweenOptions = (Ease, f64, TweenLoop, Option<LuaFunction>);

fn tween_options(options: Option<Table>) -> Result<TweenOptions, mlua::Error> {
    let mut ease = Ease::Linear;
    let mut delay = 0.0;
    let mut looping = TweenLoop::Once;
    let mut on_done = None;
    let Some(options) = options else {
        return Ok((ease, delay, looping, on_done));
    };
    for pair in options.pairs::<String, Value>() {
        let (key, value) = pair?;
        match (key.as_str(), value) {
            ("ease", Value::String(name)) => {
                let name = name.to_str()?.to_string();
                ease = Ease::parse(&name).ok_or_else(|| {
                    let names: Vec<&str> = EASES.iter().map(|(_, name)| *name).collect();
                    tween_error(format!(
                        "unknown ease '{}' (expected {})",
                        name,
                        names.join(", ")
                    ))
                })?;
            }
            ("delay", Value::Number(seconds)) => delay = duration("tween", seconds, false)?,
            ("delay", Value::Integer(seconds)) => delay = duration("tween", seconds as f64, false)?,
            ("loop", Value::String(kind)) => {
                looping = match kind.to_str()?.as_ref() {
                    "restart" => TweenLoop::Restart,
                    "ping_pong" => TweenLoop::PingPong,
                    other => {
                        return Err(tween_error(format!(
                            "unknown loop '{}' (expected restart or ping_pong)",
                            other
                        )));
                    }
                }
            }
            ("on_done", Value::Function(function)) => on_done = Some(function),
            (key, _) => {
                return Err(tween_error(format!(
                    "unknown or invalid option '{}' (expected ease, delay, loop, on_done)",
                    key
                )));
            }
        }
    }
    Ok((ease, delay, looping, on_done))
}

/// A numeric attribute value, as a number.
fn number(value: Option<&AttributeValue>) -> Option<f64> {
    match value? {
        AttributeValue::Float(value) => Some(f64::from(*value)),
        AttributeValue::Integer(value) => Some(f64::from(*value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_go_from_start_to_end() {
        for (ease, name) in EASES {
            assert!(ease.apply(0.0).abs() < 1e-9, "{}", name);
            assert!((ease.apply(1.0) - 1.0).abs() < 1e-9, "{}", name);
            assert_eq!(Ease::parse(name), Some(ease));
        }
        assert_eq!(Ease::parse("wobble"), None);
        assert!((Ease::QuadIn.apply(0.5) - 0.25).abs() < 1e-9);
        assert!(Ease::BackOut.apply(0.6) > 1.0, "overshoots");
        assert_eq!(Ease::Linear.apply(2.0), 1.0, "clamped");
    }

    #[test]
    fn loops_restart_or_turn_around() {
        assert_eq!(progress(0.5, 1.0, TweenLoop::Once), (0.5, false));
        assert_eq!(progress(1.5, 1.0, TweenLoop::Once), (1.0, true));
        assert_eq!(progress(1.25, 1.0, TweenLoop::Restart), (0.25, false));
        assert_eq!(progress(1.25, 1.0, TweenLoop::PingPong), (0.75, false));
        assert_eq!(progress(2.25, 1.0, TweenLoop::PingPong), (0.25, false));
        assert_eq!(progress(0.0, 0.0, TweenLoop::Once), (1.0, true));
    }
}
