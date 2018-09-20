.PHONY: generate
generate:
	diesel migration redo
	cargo run --release

.PHONY: init
init:
	diesel migration run
