build-linux-on-osx:
	docker run -v "$(CURDIR)":/volume -w /volume -t clux/muslrust cargo build --bin openmetrics_udpserver --target=x86_64-unknown-linux-musl --release

build: build-linux-on-osx

run:
	RUST_BACKTRACE=1 cargo run -- \
        --udp-bind=127.0.0.1:1113 \
        --http-bind=127.0.0.1:8080 \
	    --debug

example_php_client:
	cd examples/php && php client.php

example_php_server:
	cd examples/php
	php -S 0.0.0.0:3333 server.php
