//! This is a tnum implementation for Solana eBPF

use std::u64;

// This is for bit-level abstraction
#[derive(Debug, Clone, Copy)]
/// tnum definition
pub struct Tnum {
    pub value: u64,
    pub mask: u64,
}

impl Tnum {
    /// 创建实例
    pub fn new(value: u64, mask: u64) -> Self {
        Self { value, mask }
    }

    /// 创建 bottom 元素（表示"不可能的值"）
    pub fn bottom() -> Self {
        // 使用 value & mask != 0 的方式表示 bottom
        Self::new(1, 1) // 任何 value & mask != 0 的组合都是 bottom
    }

    /// 创建 top 元素（表示"任何可能的值"）
    pub fn top() -> Self {
        Self::new(0, u64::MAX)
    }

    /// 获取 value 字段
    pub fn value(&self) -> u64 {
        self.value
    }

    /// 获取 mask 字段
    pub fn mask(&self) -> u64 {
        self.mask
    }

    /// 判断是否为bottom（不可能的值）
    pub fn is_bottom(&self) -> bool {
        (self.value & self.mask) != 0
    }

    /// 判断是否为top（完全不确定的值）
    pub fn is_top(&self) -> bool {
        self.value == 0 && self.mask == u64::MAX
    }

    /// 判断是否为确定值（单点）
    pub fn is_singleton(&self) -> bool {
        self.mask == 0
    }

    /// 判断是否为非负数（最高位为0）
    pub fn is_nonnegative(&self) -> bool {
        (self.value & (1 << 63)) == 0 && (self.mask & (1 << 63)) == 0
    }

    /// 判断是否为负数（最高位为1）
    pub fn is_negative(&self) -> bool {
        (self.value & (1 << 63)) != 0 && (self.mask & (1 << 63)) == 0
    }

    /// 统计高位连续0的个数
    pub fn countl_zero(&self) -> u32 {
        self.value.leading_zeros()
    }

    /// 统计低位连续0的个数
    pub fn countr_zero(&self) -> u32 {
        self.value.trailing_zeros()
    }

    /// 统计最小的高位连续0的个数
    pub fn count_min_leading_zeros(&self) -> u32 {
        let max = self.value + self.mask;
        max.leading_zeros()
    }

    /// 统计最小的低位连续0的个数
    pub fn count_min_trailing_zeros(&self) -> u32 {
        let max = self.value + self.mask;
        max.trailing_zeros()
    }

    /// 清除高位
    pub fn clear_high_bits(&mut self, n: u32) {
        if n >= 64 {
            self.value = 0;
            self.mask = 0;
        } else {
            let mask = (1u64 << (64 - n)) - 1;
            self.value &= mask;
            self.mask &= mask;
        }
    }
}

/// 创建一个常数 tnum 实例
pub fn tnum_const(value: u64) -> Tnum {
    Tnum::new(value, 0)
}

/// from integer interval to tnum
pub fn tnum_range(min: u64, max: u64) -> Tnum {
    let chi = min ^ max;
    //最高未知位
    let bits = (64 - chi.leading_zeros()) as u64;
    //超出范围则完全未知
    if bits > 63 {
        return Tnum::new(0, u64::MAX);
    }

    //范围内的未知位
    let delta = (1u64 << bits) - 1;
    Tnum::new(min & !delta, delta)
}

/// tnum 的左移操作
pub fn tnum_lshift(a: Tnum, shift: u8) -> Tnum {
    Tnum::new(
        a.value.wrapping_shl(shift as u32),
        a.mask.wrapping_shl(shift as u32),
    )
}

/// tnum 的右移操作
pub fn tnum_rshift(a: Tnum, shift: u8) -> Tnum {
    Tnum::new(
        a.value.wrapping_shr(shift as u32),
        a.mask.wrapping_shr(shift as u32),
    )
}

/// tnum 算数右移的操作
pub fn tnum_arshift(a: Tnum, min_shift: u8, insn_bitness: u8) -> Tnum {
    match insn_bitness {
        32 => {
            //32位模式
            let value = ((a.value as i32) >> min_shift) as u32;
            let mask = ((a.mask as i32) >> min_shift) as u32;
            Tnum::new(value as u64, mask as u64)
        }
        _ => {
            //64位模式
            let value = ((a.value as i64) >> min_shift) as u64;
            let mask = ((a.mask as i64) >> min_shift) as u64;
            Tnum::new(value, mask)
        }
    }
}

/// tnum 的加法操作
pub fn tnum_add(a: Tnum, b: Tnum) -> Tnum {
    // 计算掩码之和 - 表示两个不确定数的掩码组合
    let sm = a.mask.wrapping_add(b.mask);

    // 计算确定值之和
    let sv = a.value.wrapping_add(b.value);

    // sigma = (a.mask + b.mask) + (a.value + b.value)
    // 用于检测进位传播情况
    let sigma = sm.wrapping_add(sv);

    // chi = 进位传播位图
    // 通过异或操作找出哪些位发生了进位
    let chi = sigma ^ sv;

    // mu = 最终的不确定位掩码
    // 包括:
    // 1. 进位产生的不确定性 (chi)
    // 2. 原始输入的不确定位 (a.mask | b.mask)
    let mu = chi | a.mask | b.mask;

    // 返回结果:
    // value: 确定值之和，但排除所有不确定位 (~mu)
    // mask: 所有不确定位的掩码
    Tnum::new(sv & !mu, mu)
}

/// tnum 的减法操作
pub fn tnum_sub(a: Tnum, b: Tnum) -> Tnum {
    let dv = a.value.wrapping_sub(b.value);
    let alpha = dv.wrapping_add(a.mask);
    let beta = dv.wrapping_sub(b.mask);
    let chi = alpha ^ beta;
    let mu = chi | a.mask | b.mask;
    Tnum::new(dv & !mu, mu)
}

/// tnum 的按位与操作
pub fn tnum_and(a: Tnum, b: Tnum) -> Tnum {
    let alpha = a.value | a.mask;
    let beta = b.value | b.mask;
    let v = a.value & b.value;

    Tnum::new(v, alpha & beta & !v)
}

/// tnum 的按位或操作
pub fn tnum_or(a: Tnum, b: Tnum) -> Tnum {
    let v = a.value | b.value;
    let mu = a.mask | b.mask;

    Tnum::new(v, mu & !v)
}

/// tnum 的按位异或操作
pub fn tnum_xor(a: Tnum, b: Tnum) -> Tnum {
    let v = a.value ^ b.value;
    let mu = a.mask | b.mask;

    Tnum::new(v & !mu, mu)
}

/// tnum 的乘法操作
pub fn tnum_mul(mut a: Tnum, mut b: Tnum) -> Tnum {
    let acc_v = a.value.wrapping_mul(b.value);
    let mut acc_m: Tnum = Tnum::new(0, 0);
    while (a.value != 0) || (a.mask != 0) {
        if (a.value & 1) != 0 {
            acc_m = tnum_add(acc_m, Tnum::new(0, b.mask));
        } else if (a.mask & 1) != 0 {
            acc_m = tnum_add(acc_m, Tnum::new(0, b.value | b.mask));
        }
        a = tnum_rshift(a, 1);
        b = tnum_lshift(b, 1);
    }
    tnum_add(Tnum::new(acc_v, 0), acc_m)
}

/// A constant-value optimization for tnum_mul
pub fn tnum_mul_opt(a: Tnum, b: Tnum) -> Tnum {
    // 如果一个是常数
    if a.mask == 0 && a.value.count_ones() == 1 {
        // a.value = 2 ^ x
        tnum_lshift(b, a.value.trailing_zeros() as u8)
    } else if b.mask == 0 && b.value.count_ones() == 1 {
        // a.value = 2 ^ x
        tnum_lshift(a, b.value.trailing_zeros() as u8)
    } else if (a.value | a.mask).count_ones() <= (b.value | b.mask).count_ones() {
        tnum_mul(a, b)
    } else {
        tnum_mul(b, a)
    }
}

#[test]
fn test_tnum_mul() -> () {
    let a = Tnum::new(0b100, 0b011);
    let b = Tnum::new(0b111, 0b000);
    println!("{:?}", tnum_mul(a, b));
    println!("{:?}", tnum_mul_opt(a, b));
}

///computes the join of the tnum domain.
pub fn tnum_join(a: Tnum, b: Tnum) -> Tnum {
    let v = a.value ^ b.value;
    let m = (a.mask | b.mask) | v;
    Tnum::new((a.value | b.value) & (!m), m)
}

/// [split_at_mu] splits a tnum at the first unknow.
fn split_at_mu(x: Tnum) -> (Tnum, u32, Tnum) {
    let i = x.mask.leading_ones();
    let x1 = Tnum::new(x.value >> (i + 1), x.mask >> (i + 1));
    let x2 = Tnum::new(x.value & ((1 << i) - 1), x.mask & ((1 << i) - 1));
    (x1, i, x2)
}

/// [tnum_mul_const] multiplies a constant [c] by the tnum [x]
/// which has [j] unknown bits and [n] is the fuel (Z.of_nat n = j).
fn tnum_mul_const(c: u64, x: Tnum, n: u64) -> Tnum {
    if n == 0 {
        Tnum::new(c.wrapping_mul(x.value), 0)
    } else {
        let (y1, i1, y2) = split_at_mu(x);
        let p = tnum_mul_const(c, y1, n - 1);
        let mc = Tnum::new(c.wrapping_mul(y2.mask), 0);
        let mu0 = tnum_add(tnum_lshift(p, (i1 + 1) as u8), mc);
        let mu1 = tnum_add(mu0, Tnum::new(c.wrapping_shl(i1), 0));
        tnum_join(mu0, mu1)
    }
}

/// [xtnum_mul x i y j] computes the multiplication of
/// [x]  which has [i] unknown bits by
/// [y]  which has [j] unknown bits such (i <= j)
fn xtnum_mul(x: Tnum, i: u64, y: Tnum, j: u64) -> Tnum {
    if i == 0 && j == 0 {
        Tnum::new(x.value * y.value, 0)
    } else {
        let (y1, i1, y2) = split_at_mu(y); // y = y1.mu.y2
        let p = if i == j {
            xtnum_mul(y1, j - 1, x, i)
        } else {
            xtnum_mul(x, i, y1, j - 1)
        };
        let mc = tnum_mul_const(y2.value, x, i);
        let mu0 = tnum_add(tnum_lshift(p, (i1 + 1) as u8), mc);
        let mu1 = tnum_add(mu0, tnum_lshift(x, i1 as u8));
        tnum_join(mu0, mu1)
    }
}

/// the top of the xtnum_mul
pub fn xtnum_mul_top(x: Tnum, y: Tnum) -> Tnum {
    let i = 64 - x.mask.leading_zeros() as u64;
    let j = 64 - y.mask.leading_zeros() as u64;
    if i <= j {
        xtnum_mul(x, i, y, j)
    } else {
        xtnum_mul(y, j, x, i)
    }
}

/// clear bit of n-th
fn clear_bit(num: u64, pos: u8) -> u64 {
    num & !(1 << pos)
}

/// clear bit of a tnum
fn tnum_clearbit(x: Tnum, pos: u8) -> Tnum {
    Tnum::new(clear_bit(x.value, pos), clear_bit(x.mask, pos))
}

/// bit size of a tnum
fn tnum_size(x: Tnum) -> u8 {
    let a = 64 - x.value.leading_zeros();
    let b = 64 - x.mask.leading_zeros();
    if a < b {
        b as u8
    } else {
        a as u8
    }
}

/// max 64 of a tnum
fn tnum_max(a: Tnum) -> u64 {
    a.value | a.mask
}

/// check if the pos-th of num is 0 or 1
fn testbit(num: u64, pos: u8) -> bool {
    if pos >= 64 {
        false
    } else {
        (num & (1 << pos)) != 0
    }
}

/// [xtnum_mul_high x y n] multiplies x by y
/// where n is the number of bits that are set in either x or y.
/// We also have that x <= y and 0 <= x and 0 <= y
fn xtnum_mul_high(x: Tnum, y: Tnum, n: u8) -> Tnum {
    if x.mask == 0 && y.mask == 0 {
        //if both are constants, perform normal multiplication
        Tnum::new(x.value.wrapping_mul(y.value), 0)
    } else if n == 0 {
        //panic!("should not happen");
        Tnum::new(0, 0) //should not happen
    } else {
        let b = tnum_size(y);
        let ym = testbit(y.mask, b - 1);
        let y_prime = tnum_clearbit(y, b - 1); //clear the highest bit of y
        let p = if tnum_max(y_prime) <= tnum_max(x) {
            xtnum_mul_high(y_prime, x, n - 1)
        } else {
            xtnum_mul_high(x, y_prime, n - 1)
        };
        if ym {
            tnum_join(tnum_add(p, tnum_lshift(x, b - 1)), p)
        } else {
            tnum_add(p, tnum_lshift(x, b - 1))
        }
    }
}

/// the top level of xtnum_mul_high
pub fn xtnum_mul_high_top(x: Tnum, y: Tnum) -> Tnum {
    xtnum_mul_high(
        x,
        y,
        ((x.value | x.mask).count_ones() + (y.value | y.mask).count_ones()) as u8,
    )
}

#[test]
fn test_xtnum_mul() -> () {
    let a = Tnum::new(15, 0); // 2^4 - 1
    let b = Tnum::new(0, 31); // 2^5 - 1
    println!("{:?}", tnum_mul(a, b)); // Output: Tnum { value: 0, mask: 511 } 2^(4+5) -1
    println!("{:?}", xtnum_mul_top(a, b)); // Output: Tnum { value: 0, mask: 4095 }
    println!("{:?}", xtnum_mul_high_top(a, b)); // Tnum { value: 0, mask: 511 }
}

/// aux function for tnum_mul_rec
fn tnum_decompose(a: Tnum) -> (Tnum, Tnum) {
    (
        Tnum::new(a.value >> 1, a.mask >> 1),
        Tnum::new(a.value & 1, a.mask & 1),
    )
}

/// A new tnum_mul proposed by frederic
pub fn tnum_mul_rec(a: Tnum, b: Tnum) -> Tnum {
    if a.mask == 0 && b.mask == 0 {
        // both are known
        Tnum::new(a.value * b.value, 0)
    } else if a.mask == u64::MAX && b.mask == u64::MAX {
        //both are unknown
        Tnum::new(0, u64::MAX)
    } else if (a.value == 0 && a.mask == 0) || (b.value == 0 && b.mask == 0) {
        // mult by 0
        Tnum::new(0, 0)
    } else if a.value == 1 && a.mask == 0 {
        // mult by 1
        b
    } else if b.value == 1 && b.mask == 0 {
        // mult by 1
        a
    } else {
        let (a_up, a_low) = tnum_decompose(a);
        let (b_up, b_low) = tnum_decompose(b);
        tnum_mul_rec(a_up, b_up)
        //tnum_mul_rec(a_up, b_up) + tnum_mul_rec(a_up, b_low) + tnum_mul_rec(a_low, b_up) + tnum_mul_rec(a_low, b_low)
        // TODO: this one is wrong, replace this line with the following impl
        /* decompose the mask of am && bm
        so that the last bits either 0s or 1s
        In assembly, finding the rightmost 1 or 0 of a number is fast

        let (a_up,a_low) = decompose a in
        let (b_up,b_low) = decompose b in
        // a_low and b_low are either 1s or 0s
        (mul a_up b_up) + (mul a_up b_low) +
        (mul a_low b_up) + (mul a_low b_low)
        */
    }
}

/// tnum 的交集计算
pub fn tnum_intersect(a: Tnum, b: Tnum) -> Tnum {
    let v = a.value | b.value;
    let mu = a.mask & b.mask;
    Tnum::new(v & !mu, mu)
}

/// tnum 用与截断到指定字节大小
pub fn tnum_cast(mut a: Tnum, size: u8) -> Tnum {
    //处理溢出
    a.value &= (1u64 << (size * 8)) - 1;
    a.mask &= (1u64 << (size * 8)) - 1;
    a
}

pub fn tnum_is_aligned(a: Tnum, size: u64) -> bool {
    if size == 0 {
        return true;
    } else {
        return ((a.value | a.mask) & (size - 1)) == 0;
    }
}

/// check if [b] is a subset of [a], that is
/// 1) for unknown bits: all bit-set in [b.mask] must exist in [a.mask]
/// 2) for known bits: all bit-set in [b.value] must exist in [a.value] or [a.mask]
pub fn tnum_in(a: Tnum, b: Tnum) -> bool {
    if (b.mask & !a.mask) != 0 {
        // if we find one bit-set in [b.mask] but not in [a.mask], return false
        return false;
    } else {
        // [(b.value & !a.mask)] removes all possible bit-set in [a.mask] from [b.value]
        // the rest part should be equal to [a.value]
        return a.value == (b.value & !a.mask);
    }
}

// pub fn xtnum_in(a: Tnum, b: Tnum) -> bool {
//     if (b.mask & !a.mask) != 0 {
//         return false;
//     } else {
//         return a.value == b.value;
//     }
// }

#[test]
fn test_tnum_in() -> () {
    let a = Tnum::new(1, 0);
    let b = Tnum::new(0, 1);
    println!("{:?}", tnum_in(b, a)); // true
                                     //println!("{:?}", xtnum_in(b, a)); // false
}

/// tnum转换为字符串
pub fn tnum_sbin(size: usize, mut a: Tnum) -> String {
    let mut result = vec![0u8; size];

    // 从高位到低位处理每一位
    for n in (1..=64).rev() {
        if n < size {
            result[n - 1] = match (a.mask & 1, a.value & 1) {
                (1, _) => b'x', // 不确定位
                (0, 1) => b'1', // 确定位 1
                (0, 0) => b'0', // 确定位 0
                _ => unreachable!(),
            };
        }
        // 右移处理下一位
        a.mask >>= 1;
        a.value >>= 1;
    }

    // 设置字符串结束位置
    let end = std::cmp::min(size - 1, 64);
    result[end] = 0;

    // 转换为字符串
    String::from_utf8(result[..end].to_vec()).unwrap_or_else(|_| String::new())
}

pub fn tnum_subreg(a: Tnum) -> Tnum {
    tnum_cast(a, 4)
}

pub fn tnum_clear_subreg(a: Tnum) -> Tnum {
    tnum_lshift(tnum_rshift(a, 32), 32)
}

pub fn tnum_with_subreg(reg: Tnum, subreg: Tnum) -> Tnum {
    tnum_or(tnum_clear_subreg(reg), tnum_subreg(subreg))
}

pub fn tnum_const_subreg(a: Tnum, value: u32) -> Tnum {
    tnum_with_subreg(a, tnum_const(value as u64))
}

/// 有符号取余操作（SRem）
pub fn tnum_srem(a: Tnum, b: Tnum) -> Tnum {
    // 处理 bottom 和 top 情况
    if a.is_bottom() || b.is_bottom() {
        return Tnum::bottom();
    } else if a.is_top() || b.is_top() {
        return Tnum::top();
    }

    // 处理单点值情况
    if a.is_singleton() && b.is_singleton() {
        if b.value == 0 {
            return Tnum::top(); // 除以0返回top
        }
        // 计算有符号取余
        let a_val = a.value as i64;
        let b_val = b.value as i64;
        let result = a_val % b_val;
        return Tnum::new(result as u64, 0);
    }

    // 处理除数为0的情况
    if b.value == 0 {
        return Tnum::top(); // top
    }

    // 处理除数是2的幂的情况
    if b.mask == 0
        && !((b.value >> 63) & 1 == 1)
        && ((b.value.trailing_zeros() + b.value.leading_zeros() + 1) == 64)
    {
        let low_bits = b.value - 1;
        let mut res_value = a.value & low_bits;
        let mut res_mask = a.mask & low_bits;

        // 如果被除数非负或低位0足够多
        if a.is_nonnegative() || (b.value.trailing_zeros() <= a.count_min_trailing_zeros()) {
            // 保持现有值
        }
        // 如果被除数为负且低位不全为0
        else if a.is_negative() && ((a.value & low_bits) != 0) {
            res_mask = low_bits & res_mask;
            res_value = (!low_bits) | res_value;
        }

        return Tnum::new(res_value, res_mask);
    }

    // 一般情况：结果的精度有限
    // 保留原操作数中的前导零
    let mut result = Tnum::top(); // 先创建一个top
    let leading_zeros = a.count_min_leading_zeros();
    result.clear_high_bits(leading_zeros);

    return result;
}

/// 无符号取余操作（URem）
pub fn tnum_urem(a: Tnum, b: Tnum) -> Tnum {
    // 处理 bottom 和 top 情况
    if a.is_bottom() || b.is_bottom() {
        return Tnum::bottom();
    } else if a.is_top() || b.is_top() {
        return Tnum::top();
    }

    // 处理除数为0的情况
    if b.value == 0 {
        return Tnum::top(); // 除以0返回top
    }

    // 处理低位
    // 检查除数是否为 2 的幂
    if b.mask == 0
        && !((b.value >> 63) & 1 == 1)
        && ((b.value.trailing_zeros() + b.value.leading_zeros() + 1) == 64)
    {
        // 除数是 2 的幂，直接用位掩码计算余数
        let low_bits = b.value - 1; // 例如：8-1=7(0b111)，用于掩码
        let res_value = low_bits & a.value;
        let res_mask = low_bits & a.mask;
        return Tnum::new(res_value, res_mask);
    }

    // 一般情况：结果的精度有限
    // 由于结果小于或等于任一操作数，因此操作数中的前导零在结果中也存在
    let leading_zeros = a.count_min_leading_zeros().max(b.count_min_leading_zeros());
    let mut res = Tnum::top(); // 先创建一个top
    res.clear_high_bits(leading_zeros);

    return res;
}

/// 模运算（Mod），结果总是非负
pub fn tnum_mod(a: Tnum, b: Tnum) -> Tnum {
    // 处理特殊情况
    if a.is_bottom() || b.is_bottom() {
        return Tnum::bottom();
    } else if a.is_top() || b.is_top() {
        return Tnum::top();
    }

    // 处理除数为0的情况
    if b.value == 0 {
        return Tnum::top();
    }

    // 对于非负数，mod 等同于 urem
    if a.is_nonnegative() {
        return tnum_urem(a, b);
    }

    // 对于负数，计算 srem 然后处理负结果
    let rem = tnum_srem(a, b);
    
    // 如果结果可能为负（并且除数非负），需要调整
    if rem.is_negative() && b.is_nonnegative() {
        // 如果除数是确定值，直接加上除数
        if b.is_singleton() {
            return tnum_add(rem, b);
        } else {
            // 结果范围：原来的结果和原来的结果加上除数
            return tnum_join(rem, tnum_add(rem, b));
        }
    }
    
    return rem;
}
