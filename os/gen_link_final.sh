#!/bin/bash
OUTPUT="src/link_app.S"
USER_ELF_DIR="/mnt/user/target/riscv64gc-unknown-none-elf/release"

# 收集所有 ch* 可执行文件（无后缀）
apps=()
for f in "$USER_ELF_DIR"/ch*; do
    if [ -f "$f" ] && [ ! -x "$f" ] || [ -x "$f" ]; then
        base=$(basename "$f")
        # 排除 .d 和 .bin 文件
        if [[ "$base" != *".d" && "$base" != *".bin" ]]; then
            apps+=("$base")
        fi
    fi
done

# 排序
IFS=$'\n' apps=($(sort <<<"${apps[*]}"))
unset IFS

if [ ${#apps[@]} -eq 0 ]; then
    echo "错误：在 $USER_ELF_DIR 中没有找到 ch* 可执行文件"
    echo "请先运行: cd /mnt/user && cargo build --release"
    exit 1
fi

echo "生成 $OUTPUT，包含 ${#apps[@]} 个应用"

cat > $OUTPUT << EOT
.align 3
.section .data
.global _num_app
_num_app:
    .quad ${#apps[@]}
EOT

for ((i=0; i<${#apps[@]}; i++)); do
    echo "    .quad app_${i}_start" >> $OUTPUT
done
echo "    .quad app_$((${#apps[@]}-1))_end" >> $OUTPUT

echo -e "\n.global _app_names\n_app_names:" >> $OUTPUT
for app in "${apps[@]}"; do
    echo "    .string \"$app\"" >> $OUTPUT
done

for ((i=0; i<${#apps[@]}; i++)); do
    app="${apps[$i]}"
    cat >> $OUTPUT << EOT

.section .data
.global app_${i}_start
.global app_${i}_end
.align 3
app_${i}_start:
    .incbin "${USER_ELF_DIR}/${app}"
app_${i}_end:
EOT
done

echo "完成"
