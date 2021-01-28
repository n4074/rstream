
.PHONY: frontend backend

frontend: $(rg --files --type rust frontend)
	cd frontend && wasm-pack build --dev --target web --out-name wasm --out-dir ../src/static \
	&& cp static/index.html ../src/static/

backend: $(rg --files --type rust --glob='!frontend')
	cargo build

run: frontend backend 
	cargo run

clean:
	rm src/static/*