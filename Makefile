
.PHONY: frontend backend

frontend: $(rg --files --type rust frontend)
	cd frontend && wasm-pack build --dev --target web --out-name wasm --out-dir ../src/static \
	&& cp static/index.html ../src/static/

backend: $(rg --files --type rust --glob='!frontend')
	cargo build

keymaterial: localhost.key localhost.crt
	step certificate create localhost localhost.crt localhost.key --profile self-signed  --subtle --insecure --no-password --kty=RSA --force

run: frontend backend 
	RUST_LOG=rstream=debug cargo run

clean:
	rm src/static/*