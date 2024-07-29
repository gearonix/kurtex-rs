const { core } = Deno



const kurtex = {
    test: (fn) => {
        // return core.ops.op_test(fn);
        core.print("test worked", false);
    }
}


globalThis.kurtex = kurtex