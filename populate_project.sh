# create 30 files in the test_project/entities folder, and name them as test_entity_1.json, test_entity_2.json, ..., test_entity_30.json

for i in {1..30}
do
    echo "Creating test_entity_$i.json"
    touch test_project/entities/test_entity_$i.json
done


# create 30 files in the test_project/scripts folder, and name them as test_script_1.json, test_script_2.json, ..., test_script_30.json

for i in {1..30}
do
    echo "Creating test_script_$i.json"
    touch test_project/scripts/test_script_$i.json
done

