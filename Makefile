build-linux-on-osx:
	docker run -v "$(CURDIR)":/volume -w /volume -t clux/muslrust cargo build --target=x86_64-unknown-linux-musl --release

build: build-linux-on-osx

run:
	RUST_BACKTRACE=1 cargo run -- \
            --token=some_token \
            --agent-key=some_key \
            --account-url=easybill.serverdensity.io \
            --serverdensity-endpoint=http://127.0.0.1:1337

run_from_config:
	RUST_BACKTRACE=1 cargo run -- \
            --token=some_token \
            --config=./examples/serverdensity_config_file.cfg