const { core } = Deno


const kurtex = {
    test: (fn) => {
        core.ops.op_test(fn);
        return core.print("test worked", false);
    }
}


globalThis.kurtex = kurtex