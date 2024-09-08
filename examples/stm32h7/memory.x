MEMORY
{
    FLASH    : ORIGIN = 0x08000000, LENGTH = 1M    /* BANK_1 + BANK_2 */
    RAM      : ORIGIN = 0x24000000, LENGTH = 192K+128K /* SRAM */
    RAM_D3   : ORIGIN = 0x38000000, LENGTH = 64K   /* SRAM4 */
}

SECTIONS
{
    .ram_d3 :
    {
        *(.ram_d3)
    } > RAM_D3
}
