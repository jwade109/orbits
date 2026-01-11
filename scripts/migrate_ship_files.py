import yaml
import sys
import os
import re

def iter_part_files(parts_dir):
    for folder in os.listdir(parts_dir):
        data_file = os.path.join(parts_dir, folder, "metadata.yaml")
        print(data_file)
        lines = list(open(data_file).readlines())
        out = open(data_file, "w")
        for line in lines:
            found = re.search("dims: \[(\d+), (\d+)\]", line)
            if found:
                x = int(found.group(1))
                y = int(found.group(2))
                x = int(round(x / 5))
                y = int(round(y / 5))
                new_line = f"dims: [{x}, {y}]\n"
                out.write(new_line)
            else:
                out.write(line)
        out.close()


def iter_vehicle_files(vehicles_dir):
    for filename in os.listdir(vehicles_dir):
        data_file = os.path.join(vehicles_dir, filename)
        print(data_file)
        lines = list(open(data_file).readlines())
        out = open(data_file, "w")
        for line in lines:
            found = re.search("  - (-?\d+)", line)
            if found:
                c = int(found.group(1))
                c = int(round(c / 5))
                new_line = f"  - {c}\n"
                out.write(new_line)
            else:
                out.write(line)


def main():

    parts_dir = sys.argv[1]
    vehicles_dir = sys.argv[2]

    # iter_part_files(parts_dir)
    iter_vehicle_files(vehicles_dir)



if __name__ == "__main__":
    main()
