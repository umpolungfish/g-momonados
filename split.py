import json
import math
import os

# Your updated file paths
INPUT_FILE = "/home/mrnob0dy666/imsgct/G-mOMonadOS/GB1_344.json"
OUTPUT_DIR = "/home/mrnob0dy666/imsgct/G-mOMonadOS"
CHUNK_SIZE = 1000

def find_largest_list(data):
    """Recursively searches the JSON data to extract the target array."""
    if isinstance(data, list):
        return data
    if isinstance(data, dict):
        biggest_list = []
        for value in data.values():
            found = find_largest_list(value)
            if isinstance(found, list) and len(found) > len(biggest_list):
                biggest_list = found
        return biggest_list
    return []

print(f"Opening: {INPUT_FILE}...")
try:
    with open(INPUT_FILE, 'r', encoding='utf-8') as f:
        raw_data = json.load(f)
except FileNotFoundError:
    print(f"❌ Error: Could not find the file at {INPUT_FILE}.")
    print("Please double check that the path spelling and casing matches exactly.")
    exit()

# Auto-extract the list
target_array = find_largest_list(raw_data)

if not target_array or len(target_array) <= 1:
    print("❌ Error: Could not find an array with multiple items inside this file.")
    if isinstance(raw_data, dict):
        print(f"Top-level keys present in your JSON: {list(raw_data.keys())}")
    exit()

total_items = len(target_array)
total_chunks = math.ceil(total_items / CHUNK_SIZE)
print(f"✅ Found target array containing {total_items} items.")
print(f"Chop sequence initiated: Splitting into {total_chunks} files...")

# Slice array and save chunk files
for i in range(total_chunks):
    chunk = target_array[i * CHUNK_SIZE : (i + 1) * CHUNK_SIZE]
    
    # Creates names like GB1_344_part_1.json, GB1_344_part_2.json...
    output_filename = os.path.join(OUTPUT_DIR, f"GB1_344_part_{i + 1}.json")
    
    with open(output_filename, 'w', encoding='utf-8') as out:
        json.dump(chunk, out, indent=4)

print(f"🎉 Success! Check your folder '{OUTPUT_DIR}' for the segmented files.")
