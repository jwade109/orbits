local_server:
	cargo run --release --bin server_app 5000 $$(pwd)/saves/ scenario_a

client:
	cargo run --release 127.0.0.1:5000

generate_worlds:
	cargo run --release --bin generate_worlds

