#! /usr/bin/env python3

import shutil

shutil.copytree("assets/", "target/barycenter_dist/assets/", dirs_exist_ok=True)
shutil.copytree("saves/", "target/barycenter_dist/saves/", dirs_exist_ok=True)
shutil.copy("target/release/client_app.exe", "target/barycenter_dist/barycenter.exe")

with open("target/barycenter_dist/client_with_server.bat", "w") as text_file:
    text_file.write("barycenter.exe --run-server\npause")

with open("target/barycenter_dist/client.bat", "w") as text_file:
    text_file.write("barycenter.exe\npause")
