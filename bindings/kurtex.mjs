const { core } = Deno


const kurtex = {
    test: () => {
        core.ops.op_test();
        return core.print("test worked", false);
    }
}


globalThis.kurtex = kurtex