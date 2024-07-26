arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
		build/arch/$(arch)/%.o, $(assembly_source_files))

target ?= $(arch)-trust
trust := target/$(target)/debug/libtrust.a

flags := $(CARGO_FLAGS)

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -r build

run: $(iso)
	@qemu-system-x86_64 -serial stdio -cdrom $(iso) -s

debug: $(iso)
	@qemu-system-x86_64 -serial stdio -cdrom $(iso) -s -S

test: $(test_iso)
	@qemu-system-x86_64 -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -display none $(test_iso)

dint: $(iso)
	@qemu-system-x86_64 -serial -d int -no-reboot -cdrom $(iso)

gdb:
	@rust-os-gdb/bin/rust-gdb "build/kernel-x86_64.bin" -ex "target remote :1234"

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/iso/boot/grub
	@cp $(kernel) build/iso/boot/kernel.bin
	@cp $(grub_cfg) build/iso/boot/grub
	@grub-mkrescue -o $(iso) -d /usr/lib/grub/i386-pc build/iso 2> /dev/null
	@rm -r build/iso

$(kernel): kernel $(trust) $(assembly_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(trust)

kernel:
	@RUST_TARGET_PATH=$(shell pwd) cargo build --target $(target) $(flags)

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.s
	@mkdir -p $(shell dirname $@)
	@as $< -o $@
