OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS {
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    .requests : {
        KEEP(*(.requests_start_marker .requests_start_marker.*))
        KEEP(*(.requests .requests.*))
        KEEP(*(.requests_end_marker .requests_end_marker.*))
    }
}
