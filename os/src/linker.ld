OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    /* 文本段 */
    stext = .;
    .text : {
        *(.text.entry)
        . = ALIGN(4K);
        strampoline = .;
        *(.text.trampoline);
        . = ALIGN(4K);
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    
    /* 只读数据段 */
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        
        /* 添加应用相关符号定义 */
        _app_names = .;
        LONG(0x12345678)  /* 示例值，实际应指向应用名称列表 */
        _num_app = .;
        LONG(4)            /* 示例值，实际应为应用数量 */
    }

    . = ALIGN(4K);
    erodata = .;
    
    /* 数据段 */
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    
    /* BSS段 */
    sbss_with_stack = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /* 丢弃不需要的段 */
    /DISCARD/ : {
        *(.eh_frame)
    }
}