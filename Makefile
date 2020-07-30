run:
	cargo run -- $(file) > formatter/src/main.rs
	cd formatter && cargo fmt
	batcat --paging never formatter/src/main.rs