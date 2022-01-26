use elf::elf::Elf;
use elf::header::consts::ident::class::EI_CLASS_32;
use elf::header::consts::ident::osabi::EI_OSABI_SYSTEMV;
use elf::header::consts::ident::version::EI_VERSION_CURRENT;
use elf::header::consts::machine::E_MACHINE_RISCV;
use elf::header::consts::typ::E_TYPE_ET_EXEC;
use elf::program_header::consts::typ::P_TYPE_PT_LOAD;
use elf::program_header::elf32::ProgramHeader as ProgramHeader32;
use elf::section_header::consts::typ::{
	SH_TYPE_SHT_STRTAB, SH_TYPE_SHT_SYMTAB,
};
use elf::strtab::Strtab;
use rv64gc::cpu::Cpu;
use rv64gc::mem::Memory;

const KiB: usize = 1024;
const MiB: usize = 1024 * KiB;
const GiB: usize = 1024 * MiB;

const MEM_BASE: u64 = 0x80000000;

mod tests;

#[test]
fn rv32i_tests() -> Result<(), Box<dyn std::error::Error>> {
	for dent in std::fs::read_dir(tests::BASE_RISCV_TESTS_DIR)? {
		if let Ok(dent) = dent {
			let fname = dent.file_name();
			let fname = fname.to_str().unwrap();

			if fname.starts_with("rv32ui-p-add") {
				println!("Running test: {fname}");

				let bytes = std::fs::read(dent.path())?;

				let elf = Elf::from_bytes(&bytes)?;

				dump_elf32(&elf);

				if let Elf::Elf32 { bytes, header, pheaders, sheaders } = elf {
					assert_eq!(header.e_ident.ei_class(), EI_CLASS_32);
					assert_eq!(header.e_ident.ei_osabi(), EI_OSABI_SYSTEMV);

					assert_eq!(header.e_type, E_TYPE_ET_EXEC);
					assert_eq!(header.e_machine, E_MACHINE_RISCV);
					assert_eq!(header.e_version, EI_VERSION_CURRENT);

					// TODO: load data into memory and run cpu with it

					let mut cpu = Cpu::default();
					cpu.mmu.memory = prepare_memory(bytes, &pheaders);
					cpu.pc = MEM_BASE;

					loop {
						cpu.tick();
					}

					panic!("END____");
				} else {
					panic!("Expected elf to be 32-bit but was 64-bit");
				}
			}
		}
	}

	Ok(())
}

fn dump_elf32(elf: &Elf) {
	if let Elf::Elf32 { bytes, header, pheaders, sheaders } = elf {
		println!("{:#}", header);

		let shstrtab = &sheaders[header.e_shstrndx as usize];
		let strtab =
			Strtab::new(Strtab::DEFAULT_DELIM, shstrtab.extract_data(&bytes));

		for ph in pheaders {
			println!("{:#}", ph);
		}

		for sh in sheaders {
			println!(
				"{:?} - {:#}",
				unsafe { strtab.get_str_off_unchecked(sh.sh_name as usize) },
				sh
			);
		}

		for sh in sheaders {
			if sh.sh_type == SH_TYPE_SHT_STRTAB {
				let strtab = Strtab::new(b'\0', &bytes[sh]);
				let mut idx = 0;

				while let Some(s) = unsafe { strtab.get_str_unchecked(idx) } {
					println!("{}: {}", idx, s);
					idx += 1;
				}
			}
		}

		println!("------ SYMS ---------");

		for sh in sheaders {
			if sh.sh_type == SH_TYPE_SHT_SYMTAB {
				let shstrtab = &sheaders[sh.sh_link as usize];
				let strtab = Strtab::new(b'\0', shstrtab.extract_data(&bytes));

				let symtab =
					elf::symtab::elf32::Symtab::new(EI_CLASS_32, &bytes[sh]);
				let mut idx = 0;

				while let Some(s) = symtab.get_symbol(idx) {
					println!(
						"{} - {:?}: {}",
						idx,
						unsafe {
							strtab.get_str_off_unchecked(s.st_name as usize)
						},
						s
					);
					idx += 1;
				}
			}
		}
	} else {
		panic!("Expected elf to be 32-bit but was 64-bit");
	}
}

fn prepare_memory(bytes: &[u8], pheaders: &[ProgramHeader32]) -> Memory {
	let mut mem = vec![0u8; 3 * GiB];

	for ph in pheaders {
		if ph.p_type == P_TYPE_PT_LOAD {
			let data = &bytes[ph];
			let addr = ph.p_paddr;

			let start = addr as usize;
			let end = start + (ph.p_filesz as usize);

			(&mut mem[start..end]).copy_from_slice(data);
		}
	}

	Memory(mem)
}
