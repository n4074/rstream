
.PHONY: frontend backend

frontend: $(rg --files --type rust frontend)
	cd frontend && wasm-pack build --dev --target web --out-name wasm --out-dir ../src/static \
	&& cp static/index.html ../src/static/ \
	&& cp static/main.css ../src/static/ \
	&& cp static/test.js ../src/static/

backend: $(rg --files --type rust --glob='!frontend')
	cargo build

keymaterial: localhost.key localhost.crt
	#step certificate create localhost localhost.crt localhost.key --profile self-signed  --subtle --insecure --no-password --kty=RSA --force
	step certificate create nostalgiaforinfinity.local localhost.crt localhost.key --profile self-signed  --subtle --insecure --no-password --kty=RSA --force

run: frontend backend 
	RUST_LOG=rstream=info,frontend=info cargo run -- -h 0.0.0.0

clean:
	rm src/static/*