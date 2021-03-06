use wasm::instance::{Instance, VmCtx, get_function_addr};
use wasm::{Module, ModuleEnvironment, DataInitializer};
use wasm::compilation::TrapData;
use memory::{Region, MemFlags};
use nabi::{Result, Error};
use core::mem;
use alloc::Vec;
use nil::{Ref, HandleRef};
use cretonne_codegen::settings::{self, Configurable};
use cretonne_codegen::ir::TrapCode;
use cretonne_wasm::translate_module;
use cretonne_native;

/// A `Wasm` represents
/// webassembly code compiled
/// into machine code. You must
/// have one of these to create
/// a process.
#[allow(dead_code)]
#[derive(HandleRef)]
pub struct Wasm {
    data_initializers: Vec<DataInitializer>,
    functions: Vec<usize>,
    traps: Vec<TrapData>,
    module: Module,
    region: Region,
    start_func: extern fn(&VmCtx),
}

impl Wasm {
    /// Compile webassembly bytecode into a Wasm.
    pub fn compile(wasm: &[u8]) -> Result<Ref<Wasm>> {
        let (mut flag_builder, isa_builder) = cretonne_native::builders()
            .map_err(|_| internal_error!())?;

        flag_builder.set("opt_level", "best")
            .map_err(|_| internal_error!())?;

        let isa = isa_builder.finish(settings::Flags::new(flag_builder));

        let module = Module::new();
        let mut environ = ModuleEnvironment::new(isa.flags(), module);

        translate_module(wasm, &mut environ)
            .map_err(|_| internal_error!())?;

        let translation = environ.finish_translation();
        let (compliation, module, data_initializers) = translation.compile(&*isa)?;

        compliation.emit(module, data_initializers)
    }

    /// Used for internal use.
    pub fn new(
        module: Module,
        data_initializers: Vec<DataInitializer>,
        mut region: Region,
        start_func: *const (),
        functions: Vec<usize>,
        traps: Vec<TrapData>,
    )
        -> Result<Ref<Wasm>>
    {
        let flags = MemFlags::READ | MemFlags::EXEC;
        region.remap(flags)?;

        let start_func = unsafe {
            mem::transmute(start_func)
        };

        Ref::new(Wasm {
            data_initializers,
            module,
            functions,
            region,
            start_func,
            traps,
        })
    }

    pub fn generate_instance(&self) -> Result<Instance> {
        let code_base = self.region.start().as_ptr() as _;
        Instance::build(&self.module, &self.data_initializers, code_base, &self.functions)
    }

    pub fn start_func(&self) -> extern fn(&VmCtx) {
        self.start_func
    }

    pub fn lookup_func(&self, function_index: usize) -> Result<*const ()> {
        let base = self.region.as_ptr() as _;
        if function_index < self.functions.len() {
            Ok(get_function_addr(base, &self.functions, function_index))
        } else {
            Err(Error::OUT_OF_BOUNDS)
        }
    }

    pub fn lookup_trap_code(&self, inst: *const ()) -> Option<TrapCode> {
        let offset = (inst as *const u8) as usize - self.region.start().as_ptr::<u8>() as usize;
        self.traps.iter()
            .find(|trap_data| trap_data.offset == offset)
            .map(|trap_data| trap_data.code)
    }

    /// Returns the index of the specified function in the module function index space.
    pub fn lookup_func_index(&self, addr: *const ()) -> Option<usize> {
        let base = self.region.as_ptr() as _;
    
        self.functions
            .iter()
            .enumerate()
            .find(|&(index, _)| {
                get_function_addr(base, &self.functions, index) == addr
            })
            .map(|(i, _)| i)
    }

    pub fn module(&self) -> &Module {
        &self.module
    }
}
