isopath := "build/os-{{arch}}.iso"
arch  := "x86_64"

grub_cfg := "src/arch/" + arch + "/grub.cfg"

features := ""
feature_flag := if features != "" { "--features " + features } else { "" }

clean:
    @rm -r build

run params="":
    @cargo run {{params}} {{feature_flag}}

debug params="":
    @cargo run {{params}} {{feature_flag}}

test params="":
    @cargo test {{params}} {{feature_flag}}

_run kernel_bin: (iso kernel_bin)
    @qemu-system-{{arch}} -serial stdio -cdrom {{isopath}} -s

_debug kernel_bin: (iso kernel_bin)
    @qemu-system-{{arch}} -serial stdio -cdrom {{isopath}} -s -S

_test kernel_bin: (iso kernel_bin)
    @qemu-system-{{arch}} -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -display none -cdrom {{isopath}} || if [ $? -eq 33 ]; then exit 0; else exit 1; fi

_dint kernel_bin: (iso kernel_bin)
    @qemu-system-{{arch}} -serial -d int -no-reboot -cdrom {{isopath}}

gdb:
    @gdb $(KERNEL_BIN) -ex "target remote :1234"

iso kernel_bin:
    @mkdir -p build/iso/boot/grub
    @cp {{kernel_bin}} build/iso/boot/kernel.bin
    @cp {{grub_cfg}} build/iso/boot/grub
    @grub-mkrescue -o {{isopath}} -d /usr/lib/grub/i386-pc build/iso 2> /dev/null
    @rm -r build/iso
