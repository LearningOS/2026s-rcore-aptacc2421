#!/bin/bash
OUTPUT="src/link_app.S"
USER_ELF_DIR="/mnt/user/target/riscv64gc-unknown-none-elf/release"

apps=()
for f in "$USER_ELF_DIR"/ch*; do
    if [ -f "$f" ] && [ ! -x "$f" ] || [ -x "$f" ]; then
        base=$(basename "$f")
        if [[ "$base" != *".d" && "$base" != *".bin" ]]; then
            apps+=("$base")
        fi
    fi
done

IFS=$'\n' apps=($(sort <<<"${apps[*]}"))
unset IFS

if [ ${#apps[@]} -eq 0 ]; then
    echo "错误：未找到 ch* 可执行文件"
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
