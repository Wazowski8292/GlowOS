.PHONY: build run check

clean: 
	cd kernel && cargo clean
	cd image-builder && cargo clean

build:
	cd kernel && cargo build
	cd image-builder && cargo build

check:
	cd kernel && cargo check
	cd image-builder && cargo check

run: build
	cd image-builder && cargo run -- ../kernel/target/x86_64-os/debug/os
	./image-builder/qemu-run.sh

test:
	cd kernel && cargo test --no-run --message-format=json --test-threads=1 2>&1 \
		| python3 -c "import sys,json; [print(o['executable']) for l in sys.stdin if (o:=json.loads(l)).get('executable') and o.get('reason')=='compiler-artifact']" \
		| xargs -I{} cargo run --manifest-path ../image-builder/Cargo.toml -- {}