local_server:
	cargo run --release --bin server_app 5000 $$(pwd)/saves/ test_world

client:
	cargo run --release --bin test_app
