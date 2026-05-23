server:
	cargo run --release --bin server_app $$(pwd)/saves/ test_world

client:
	cargo run --release --bin test_app
