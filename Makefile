build:
	cargo build --release

local_server:
	cargo run --release --bin server_app 5000 $$(pwd)/saves/ scenario_b

client:
	cargo run --release -- -s saves/scenario_b -a 127.0.0.1:5000

generate_worlds:
	cargo run --release --bin generate_worlds
