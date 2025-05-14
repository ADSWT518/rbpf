// 文件名: src/compare.rs
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use solana_sbpf::tnum::{tnum_in, Tnum};

// 定义方法名称
const METHOD_NAMES: [&str; 5] = [
    "C_tnum_mul",
    "tnum_mul",
    "tnum_mul_opt",
    "xtnum_mul_top",
    "xtnum_mul_high_top",
];

// 统计信息结构体
struct MethodStats {
    method: String,
    equal: u32,
    less_than: u32,
    more_than: u32,
    not_equal: u32,
    total_count: u32,
    total_time: f64,
    avg_time: f64,
}

impl MethodStats {
    fn new(method: &str) -> Self {
        MethodStats {
            method: method.to_string(),
            equal: 0,
            less_than: 0,
            more_than: 0,
            not_equal: 0,
            total_count: 0,
            total_time: 0.0,
            avg_time: 0.0,
        }
    }
}

// 不一致结果结构体
#[derive(Serialize)]
struct Inconsistency {
    case_number: u32,
    input_a: TnumValue,
    input_b: TnumValue,
    c_output: TnumValue,
    rust_output: TnumValue,
    method: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TnumValue {
    value: u64,
    mask: u64,
}

#[derive(Deserialize)]
struct TestCase {
    input_a: TnumValue,
    input_b: TnumValue,
    results: Vec<MethodResult>,
}

#[derive(Deserialize)]
struct MethodResult {
    method: String,
    output: TnumValue,
    avg_time_ns: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <c_test_results.json>", args[0]);
        return Err("Insufficient arguments".into());
    }

    let input_file = &args[1];

    // 读取JSON文件
    let json_data = fs::read_to_string(input_file)?;
    let test_cases: Vec<TestCase> = serde_json::from_str(&json_data)?;

    println!("分析 {} 个测试用例...", test_cases.len());

    // 初始化统计数据
    let mut stats: Vec<MethodStats> = METHOD_NAMES
        .iter()
        .map(|&method| MethodStats::new(method))
        .collect();

    // 不一致结果存储
    let mut inconsistencies: Vec<Inconsistency> = Vec::new();

    // 遍历所有测试用例
    for (i, test_case) in test_cases.iter().enumerate() {
        // 获取输入值
        let input_a = &test_case.input_a;
        let input_b = &test_case.input_b;

        // 先找C_tnum_mul的结果作为基准
        let mut c_result = None;
        let mut c_output_value = 0;
        let mut c_output_mask = 0;
        let mut c_time = 0.0;

        for result in &test_case.results {
            if result.method == "C_tnum_mul" {
                c_result = Some(result);
                c_output_value = result.output.value;
                c_output_mask = result.output.mask;
                c_time = result.avg_time_ns;

                // 更新C_tnum_mul的统计信息
                for stat in &mut stats {
                    if stat.method == "C_tnum_mul" {
                        stat.total_count += 1;
                        stat.equal += 1; // C和自己比总是正确的
                        stat.total_time += c_time;
                        break;
                    }
                }
                break;
            }
        }

        // 如果没有找到C_tnum_mul的结果，跳过
        if c_result.is_none() {
            continue;
        }

        // 遍历所有其他方法的结果，与C_tnum_mul比较
        for result in &test_case.results {
            // 跳过C_tnum_mul自己
            if result.method == "C_tnum_mul" {
                continue;
            }

            // 比较结果
            let correct = result.output.value == c_output_value && result.output.mask == c_output_mask;

            // 更新统计信息
            for stat in &mut stats {
                if stat.method == result.method {
                    stat.total_count += 1;
                    stat.total_time += result.avg_time_ns;

                    if correct {
                        stat.equal += 1;
                    } else {
                        // 记录不一致结果
                        if tnum_in(Tnum::new(result.output.value, result.output.mask), Tnum::new(c_output_value, c_output_mask)) {
                            stat.less_than += 1;
                        } else  if tnum_in(Tnum::new(c_output_value, c_output_mask), Tnum::new(result.output.value, result.output.mask)) {
                            stat.more_than += 1;
                        }else {
                            stat.not_equal += 1;
                        }
                        inconsistencies.push(Inconsistency {
                            case_number: (i + 1) as u32,
                            input_a: input_a.clone(),
                            input_b: input_b.clone(),
                            c_output: TnumValue {
                                value: c_output_value,
                                mask: c_output_mask,
                            },
                            rust_output: result.output.clone(),
                            method: result.method.clone(),
                        });
                    }
                    break;
                }
            }
        }

        // 显示进度
        // if (i + 1) % 10 == 0 || i == test_cases.len() - 1 {
        //     print!("\r处理进度: {}/{} ({:.1}%)", 
        //         i + 1, 
        //         test_cases.len(), 
        //         (i + 1) as f64 / test_cases.len() as f64 * 100.0);
        //     io::stdout().flush()?;
        // }
    }

    println!("\n");

    // 计算平均时间
    for stat in &mut stats {
        if stat.total_count > 0 {
            stat.avg_time = stat.total_time / stat.total_count as f64;
        }
    }

    // 打印统计结果
    println!("{:<24} {:<18} {:<18} {:<18} {:<18} {:<18}", "method", "average time(ns)", "equal","less than", "more than","not_equal");
    println!("------------------------------------------------------------------------");

    for stat in &stats {
        if stat.total_count > 0 {
            let equal = stat.equal as f64 / stat.total_count as f64 * 100.0;
            let less_than = stat.less_than as f64 / stat.total_count as f64 * 100.0;
            let more_than = stat.more_than as f64 / stat.total_count as f64 * 100.0;
            let not_equal = stat.not_equal as f64 / stat.total_count as f64 * 100.0;
            println!(
                "{:<24} {:<18.1} {:<18.1} {:<18.1} {:<18.1} {:<18.1}",
                stat.method, stat.avg_time, equal, less_than, more_than, not_equal
            );
        }
    }

    // 如果有不一致结果，将它们保存到JSON文件
    if !inconsistencies.is_empty() {
        let json_output = serde_json::to_string_pretty(&inconsistencies)?;
        let filename = "inconsistencies.json";
        let mut file = File::create(filename)?;
        file.write_all(json_output.as_bytes())?;
        println!("\n不一致结果已保存到: {}", filename);
    } else {
        println!("\n所有实现的结果与C_tnum_mul完全一致！");
    }

    Ok(())
}