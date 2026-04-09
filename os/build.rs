use std::fs::{read_dir, File};
use std::io::{Result, Write};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed=../user/target/riscv64gc-unknown-none-elf/release/");
    match insert_app_data() {
        Ok(_) => println!("cargo:warning=link_app.S generated successfully"),
        Err(e) => println!("cargo:warning=Failed to generate link_app.S: {}", e),
    }
}

static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn insert_app_data() -> Result<()> {
    let target_dir = Path::new(TARGET_PATH);
    if !target_dir.exists() {
        // 如果目录不存在，可能是用户程序尚未编译，此时创建一个空的 link_app.S 以避免编译失败
        create_empty_link_app()?;
        return Ok(());
    }

    let entries = match read_dir(target_dir) {
        Ok(entries) => entries,
        Err(_) => {
            create_empty_link_app()?;
            return Ok(());
        }
    };

    let mut apps: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            // 只取 ch 开头的文件，排除 .d 后缀
            if file_name.starts_with("ch") && !file_name.ends_with(".d") && path.is_file() {
                Some(file_name.to_string())
            } else {
                None
            }
        })
        .collect();

    if apps.is_empty() {
        create_empty_link_app()?;
        return Ok(());
    }

    apps.sort();

    let mut f = File::create("src/link_app.S")?;

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    writeln!(
        f,
        r#"
    .global _app_names
_app_names:"#
    )?;
    for app in &apps {
        writeln!(f, r#"    .string "{}""#, app)?;
    }

    for (idx, app) in apps.iter().enumerate() {
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
    .align 3
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
    }
    Ok(())
}

fn create_empty_link_app() -> Result<()> {
    let mut f = File::create("src/link_app.S")?;
    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 0
    .quad app_0_end
    .global _app_names
_app_names:
    .string ""
"#
    )?;
    Ok(())
}