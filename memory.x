MEMORY
{
  /*
   * Start at the $loadaddr in u-boot for VisionFive2
   * 
   * This allows to boot into the u-boot console,
   * and use the following commands to load into memory:
   *
   * # load mmc 1:2 $loadaddr example.bin
   * # go $loadaddr
   * 
   * The first command loads an exmaple firmware `example.bin`
   * from the second partition of an sdcard at slot 1
   *
   * The second command executes the binary located at $loadaddr.
   */
  RAM : ORIGIN = 0x60000000, LENGTH = 0x8000000
}

REGION_ALIAS("REGION_TEXT", RAM);
REGION_ALIAS("REGION_RODATA", RAM);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);
