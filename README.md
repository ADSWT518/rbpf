# solana-sbpf

SBPF virtual machine + A better verifier

## 🚀 Latest Updates

**最新测试项目已转移至**: [https://github.com/shenghaoyuan/rbpf/tree/tnum_test](https://github.com/shenghaoyuan/rbpf/tree/tnum_test)

该分支包含了最新的 tnum 抽象域实现和 wrapped interval 域的完整 Rust 实现。主要更新包括：
- 完整的 wrapped interval arithmetic 实现
- 所有算术运算、join/meet 操作
- 全面的测试套件和演示程序
- 基于论文 "A Wrapped Interval Arithmetic" 的忠实实现

## Our Verifier

- tnum abstract domain (see `src/tnum.rs`)
- `tnum_mul` performance testing (see `tests/tnum_mul.rs` and `tests/tnum_mul.c`)

To check the performance of mulpti-tnum-mul, just do
```shell
$ sudo apt install libjson-c-dev #install json lib for C programs

$ make test (N=100 ITERATION=1000)
...
Total:
method                average time(ns)   equal           less than       more than       not_equal         
-------------------------------------------------------------------------------------------------
C_tnum_mul            128.0              100.0           0.0             0.0             0.0               
tnum_mul              57.9               100.0           0.0             0.0             0.0               
tnum_mul_opt          63.0               100.0           0.0             0.0             0.0               
xtnum_mul_top         709.5              0.0             98.0            1.0             1.0               
xtnum_mul_high_top    159.0              14.0            11.0            70.0            5.0 
```
* where `accuracy` represents: if the result of other mul functions is same to `tnum_mul`, then we think it is correct, otherwise incorrect. `accuracy` could be improved using the following four cases:

```rust
// equal, less_than, more_than, not_equal
let ra = tnum_mul a b;
let rb = other_tmum_mul a b;
if ra == rb {
   equal += 1
} else if tnum_in rb ra { // ra in rb
   less_than + = 1
} else if tnum_in ra rb { // rb in ra
   more_than + = 1
} else {
   not_equal + = 1
}
```

## SBPF VM
see [README_OLD](README_OLD.md)
