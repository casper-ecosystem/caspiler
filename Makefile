run:
	cargo run -- -O none $(file) > formatter/src/main.rs
	cd formatter && cargo fmt
	batcat --paging never formatter/src/main.rs