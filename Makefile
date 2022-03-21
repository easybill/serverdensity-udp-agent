build-linux-on-osx:
	docker run -v "$(CURDIR)":/volume -w /volume -t clux/muslrust cargo build --bin serverdensity_udpserver --target=x86_64-unknown-linux-musl --release

build: build-linux-on-osx

run:
	RUST_BACKTRACE=1 cargo run -- \
            --token=some_token \
            --agent-key=some_key \
            --account-url=easybill.serverdensity.io \
            --serverdensity-endpoint=http://127.0.0.1:3333 \
	    --debug

run_from_config:
	RUST_BACKTRACE=1 cargo run -- \
            --token=some_token \
            --config=./examples/serverdensity_config_file.cfg


example_php_client:
	cd examples/php && php client.php

example_php_server:
	cd examples/php
	php -S 0.0.0.0:3333 server.php
