arch ?= x86_64
iso := build/os-$(arch).iso

grub_cfg := src/arch/$(arch)/grub.cfg

clean:
	@rm -r build

run: $(iso)
	@qemu-system-$(arch) -serial stdio -cdrom $(iso) -s

debug: $(iso)
	@qemu-system-$(arch) -serial stdio -cdrom $(iso) -s -S

test: $(iso)
	@qemu-system-$(arch) -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -display none -cdrom $(iso)

dint: $(iso)
	@qemu-system-$(arch) -serial -d int -no-reboot -cdrom $(iso)

gdb:
	@gdb $(KERNEL_BIN) -ex "target remote :1234"

iso: $(iso)

$(iso): $(KERNEL_BIN) $(grub_cfg)
	@mkdir -p build/iso/boot/grub
	@cp $(KERNEL_BIN) build/iso/boot/kernel.bin
	@cp $(grub_cfg) build/iso/boot/grub
	@grub-mkrescue -o $(iso) -d /usr/lib/grub/i386-pc build/iso 2> /dev/null
	@rm -r build/iso
