//! 自动测试覆盖率分析器
//!
//! 功能：
//! 1. 扫描所有模块的 .md 文档，提取"测试方案"
//! 2. 分析现有 .rs 文件的测试代码
//! 3. 计算覆盖率差距
//! 4. 生成缺失的测试代码待审核

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use walkdir::WalkDir;

struct ModuleInfo {
    name: String,
    md_path: PathBuf,
    rs_path: PathBuf,
    test_cases: Vec<String>,
    existing_tests: Vec<String>,
}

fn main() {
    println!("=== 自动测试覆盖率分析器 ===");

    let datastate_path = Path::new("crates/datastate/src");
    let pending_dir = Path::new("docs/task/test/pending");
    fs::create_dir_all(pending_dir).expect("创建待审核目录失败");

    // 收集所有模块信息
    let mut modules = scan_modules(datastate_path);

    println!("扫描到 {} 个模块", modules.len());

    // 提取测试方案
    extract_test_cases(&mut modules);

    // 统计现有测试
    analyze_existing_tests(&mut modules);

    // 计算覆盖率
    let total_cases = modules.iter().map(|m| m.test_cases.len()).sum::<usize>();
    let total_existing = modules.iter().map(|m| m.existing_tests.len()).sum::<usize>();
    let coverage = if total_cases > 0 {
        (total_existing as f64 / total_cases as f64) * 100.0
    } else {
        0.0
    };

    println!("\n=== 覆盖率报告 ===");
    println!("总测试用例数: {}", total_cases);
    println!("已实现测试数: {}", total_existing);
    println!("缺失测试数: {}", total_cases - total_existing);
    println!("当前覆盖率: {:.2}%", coverage);
    println!("目标覆盖率: 80%");

    // 生成缺失测试代码
    let missing_count = generate_missing_tests(&modules, pending_dir);
    println!("\n生成缺失测试代码: {} 个", missing_count);

    // 创建审核清单
    create_review_list(&modules, pending_dir, coverage, total_cases, total_existing, missing_count);

    println!("\n✅ 任务完成！");
    println!("待审核测试文件位于: docs/task/test/pending/");
    println!("请人工审核后手动提交");
}

fn scan_modules(path: &Path) -> Vec<ModuleInfo> {
    let mut modules = Vec::new();

    // 遍历所有子目录
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name().to_str().map(|s| s.ends_with(".md")).unwrap_or(false) {
            let md_path = entry.path().to_path_buf();
            let file_name = md_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // 排除不需要的文档
            if file_name == "README" || file_name == "README_CN" || file_name == "refactor_suggestions" {
                continue;
            }

            // 查找对应的 .rs 文件
            if let Some(rs_path) = find_rs_file(path, entry.path()) {
                modules.push(ModuleInfo {
                    name: file_name.to_string(),
                    md_path: md_path.clone(),
                    rs_path: rs_path,
                    test_cases: Vec::new(),
                    existing_tests: Vec::new(),
                });
            }
        }
    }

    modules
}

fn find_rs_file(base_path: &Path, md_path: &Path) -> Option<PathBuf> {
    // 尝试在相同目录下找同名的 .rs 文件
    let mut rs_path = md_path.with_extension("rs");
    if rs_path.exists() {
        return Some(rs_path);
    }

    // 如果是在子目录，尝试在父目录找
    if let Some(parent) = md_path.parent() {
        let file_name = md_path.file_stem().and_then(|s| s.to_str())?;
        rs_path = parent.join(format!("{}.rs", file_name));
        if rs_path.exists() {
            return Some(rs_path);
        }
    }

    None
}

fn extract_test_cases(modules: &mut [ModuleInfo]) {
    let re_section = Regex::new(r"(?s)## 测试方案\n(.*?)(?=\n##|\z)").unwrap();
    let re_test = Regex::new(r"(?m)^\s*[*-]\s*\*\*(.+?)\*\*:(.+)$").unwrap();

    for module in modules {
        if let Ok(content) = fs::read_to_string(&module.md_path) {
            if let Some(captures) = re_section.captures(&content) {
                let test_section = captures.get(1).unwrap().as_str();

                for line in test_section.lines() {
                    if let Some(m) = re_test.captures(line) {
                        let test_name = m.get(1).unwrap().as_str().trim().to_string();
                        if !test_name.is_empty() {
                            module.test_cases.push(test_name);
                        }
                    }
                }
            }
        }

        if module.test_cases.is_empty() {
            println!("⚠️  模块 {} 未找到测试方案", module.name);
        } else {
            println!("✓ 模块 {}: 提取到 {} 个测试用例", module.name, module.test_cases.len());
        }
    }
}

fn analyze_existing_tests(modules: &mut [ModuleInfo]) {
    let re_test_fn = Regex::new(r"#\[test\]\s*(?:#\[[^\]]+\]\s*)*fn\s+(test_\w+)\s*\(").unwrap();

    for module in modules {
        if module.rs_path.exists() {
            if let Ok(content) = fs::read_to_string(&module.rs_path) {
                for cap in re_test_fn.captures_iter(&content) {
                    if let Some(name) = cap.get(1) {
                        module.existing_tests.push(name.as_str().to_string());
                    }
                }
            }
        }

        println!("  模块 {}: 已有 {} 个测试函数", module.name, module.existing_tests.len());
    }
}

fn generate_missing_tests(modules: &[ModuleInfo], pending_dir: &Path) -> usize {
    let mut total_generated = 0;

    for module in modules {
        if module.test_cases.is_empty() {
            continue;
        }

        // 找出缺失的测试
        let missing: Vec<String> = module.test_cases.iter()
            .filter(|case| {
                let test_name = format!("test_{}_{}", module.name, case.to_lowercase().replace(' ', "_").replace(|c: char| !c.is_alphanumeric() && c != '_', ""));
                !module.existing_tests.contains(&test_name)
            })
            .collect();

        if missing.is_empty() {
            println!("✓ 模块 {}: 无缺失测试", module.name);
            continue;
        }

        println!("⚠️  模块 {}: 缺失 {} 个测试", module.name, missing.len());

        // 生成测试代码
        let file_name = format!("{}_{}_{}.rs",
            "datastate",
            module.name,
            chrono::Local::now().format("%Y%m%d").to_string()
        );
        let file_path = pending_dir.join(file_name);

        let mut code = String::new();
        code.push_str("//! 自动生成的测试代码 - 待审核\n");
        code.push_str("//! 请验证测试逻辑的正确性后手动提交\n\n");

        // 添加 use 语句
        code.push_str("use super::*;\n");
        code.push_str("use base::mylogger::mylogger;\n");
        code.push_str("use std::collections::HashMap;\n\n");

        for test_case in &missing {
            let test_name = format!("test_{}_{}",
                module.name.to_lowercase(),
                test_case.to_lowercase().replace(' ', "_").replace(|c: char| !c.is_alphanumeric() && c != '_', "")
            );

            code.push_str(&format!("#[test]\n"));
            code.push_str(&format!("fn {}() {{\n", test_name));
            code.push_str("    // TODO: 实现测试逻辑\n");
            code.push_str(&format!("    // 测试用例: {}\n", test_case));
            code.push_str("    // Arrange\n");
            code.push_str("    // let logger = mylogger!();\n");
            code.push_str("    \n");
            code.push_str("    // Act\n");
            code.push_str("    // \n");
            code.push_str("    // Assert\n");
            code.push_str("    // \n");
            code.push_str("}\n\n");
        }

        fs::write(&file_path, code).expect("写入测试文件失败");
        println!("  生成测试文件: {}", file_path.display());
        total_generated += missing.len();
    }

    total_generated
}

fn create_review_list(
    modules: &[ModuleInfo],
    pending_dir: &Path,
    coverage: f64,
    total_cases: usize,
    total_existing: usize,
    missing_count: usize
) {
    let list_path = pending_dir.join("review_list.md");

    let mut content = String::new();
    content.push_str("# 测试覆盖率审核清单\n\n");
    content.push_str(&format!("生成时间: {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    content.push_str(&format!("项目: datastate\n"));
    content.push_str(&format!("目标路径: crates/datastate/src\n"));
    content.push_str(&format!("目标覆盖率: 80%\n\n"));

    content.push_str("## 覆盖率报告\n\n");
    content.push_str(&format!("- 总测试用例数: {}\n", total_cases));
    content.push_str(&format!("- 已实现测试数: {}\n", total_existing));
    content.push_str(&format!("- 缺失测试数: {}\n", missing_count));
    content.push_str(&format!("- 当前覆盖率: {:.2}%\n", coverage));
    content.push_str(&format!("- 状态: {}\n\n",
        if coverage >= 80.0 { "✅ 达标" } else { "⚠️  需补充" }
    ));

    content.push_str("## 模块详情\n\n");
    content.push_str("| 模块名 | 总用例 | 已实现 | 缺失 | 覆盖率 | 状态 |\n");
    content.push_str("|--------|--------|--------|------|--------|------|\n");

    for module in modules {
        let module_total = module.test_cases.len();
        let module_existing = module.existing_tests.len();
        let module_missing = module_total - module_existing;
        let module_coverage = if module_total > 0 {
            (module_existing as f64 / module_total as f64) * 100.0
        } else {
            0.0
        };

        content.push_str(&format!("| {} | {} | {} | {} | {:.1}% | {} |\n",
            module.name,
            module_total,
            module_existing,
            module_missing,
            module_coverage,
            if module_missing > 0 { "⚠️" } else { "✅" }
        ));
    }

    content.push_str("\n## 生成的测试文件\n\n");

    // 列出生成的测试文件
    let entries = fs::read_dir(pending_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|s| s == "rs").unwrap_or(false));

    for entry in entries {
        content.push_str(&format!("- `{}`\n", entry.path().display()));
    }

    content.push_str("\n## 审核指示\n\n");
    content.push_str("### 审核步骤\n\n");
    content.push_str("1. ✅ 检查测试命名规范是否符合 `test_{模块名}_{用例名}`\n");
    content.push_str("2. ✅ 验证测试代码逻辑是否正确实现了测试用例描述\n");
    content.push_str("3. ✅ 确认使用 Arrange-Act-Assert 结构\n");
    content.push_str("4. ✅ 检查是否包含必要的 `use` 声明\n");
    content.push_str("5. ✅ 确保无 MOCK 数据，使用真实数据和 API 调用\n");
    content.push_str("6. ✅ 验证测试可编译并通过\n\n");

    content.push_str("### 通过标准\n\n");
    content.push_str("- [ ] 所有测试命名规范正确\n");
    content.push_str("- [ ] 测试逻辑覆盖所有测试用例\n");
    content.push_str("- [ ] 测试代码无语法错误\n");
    content.push_str("- [ ] `cargo test` 全部通过\n");
    content.push_str("- [ ] 覆盖率提升至 80% 以上\n\n");

    content.push_str("### 提交方式\n\n");
    content.push_str("审核通过后，由**人工手动**执行：\n");
    content.push_str("```bash\n");
    content.push_str("git add docs/task/test/pending/*.rs\n");
    content.push_str("git commit -m \"feat: 自动生成测试覆盖率提升\"\n");
    content.push_str("git push\n");
    content.push_str("```\n\n");

    content.push_str("---\n");
    content.push_str("*本清单由自动测试覆盖率分析器生成*\n");

    fs::write(list_path, content).expect("写入审核清单失败");
    println!("生成审核清单: {}", list_path.display());
}
