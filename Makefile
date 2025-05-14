CC = gcc
CFLAGS = -Wall -Wextra -I./include -o2
LDFLAGS = -ljson-c

SRC_DIR = tests
BUILD_DIR = ./tests/build
RUST_JSON = ./tests/build/rust_test_cases.json
C_JSON = ./tests/build/c_test_results.json

N ?= 100
ITERATIONS ?= 100

# 创建构建目录
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# 编译tnum.c
$(BUILD_DIR)/tnum.o: $(SRC_DIR)/tnum.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

# 编译tnum_mul.c
$(BUILD_DIR)/tnum_mul: $(SRC_DIR)/tnum_mul.c $(BUILD_DIR)/tnum.o | $(BUILD_DIR)
	$(CC) $(CFLAGS) $^ -o $@ $(LDFLAGS)

# 生成测试用例（运行test.rs）
$(RUST_JSON):
	cargo run --release --bin test_mul -- $(ITERATIONS) $(N)

# 执行完整测试流程
test: build rust-test c-test compare-results

build: $(BUILD_DIR)

# 运行Rust测试
rust-test: $(RUST_JSON)
	@echo "Rust测试完成，生成测试用例：$(RUST_JSON)"

# 运行C实现测试
c-test: $(BUILD_DIR)/tnum_mul rust-test
	$(BUILD_DIR)/tnum_mul $(RUST_JSON) $(ITERATIONS)
	@echo "C_tnum_mul测试完成，生成结果：$(C_JSON)"

# 比较结果
compare-results: $(C_JSON)
	@echo "比较测试结果..."
	cargo run --release --bin compare -- $(C_JSON)
	@echo "测试比较完成"

# 清理
clean:
	rm -rf $(BUILD_DIR) $(RUST_JSON) $(C_JSON)
	cargo clean

# 显示帮助
help:
	@echo "使用说明:"
	@echo "  make test [N=100] [ITERATIONS=100]      - 执行完整测试流程"
	@echo "  make rust-test [N=100] [ITERATIONS=100] - 只运行Rust测试生成用例"
	@echo "  make c-test [ITERATIONS=100]            - 运行C实现测试"
	@echo "  make clean   				   - 清理生成的文件"

.PHONY: test rust-test c-test compare-results clean help