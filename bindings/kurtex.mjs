const { core } = Deno


const kurtex = {
    test: () => {
        core.ops.op_test();
        return core.print("test worked", false);
    }
}


function registerApiGlobally() {
    Object.entries(kurtex).forEach(([key, value]) => {
        globalThis[key] = value
    })
}

registerApiGlobally()


globalThis.kurtex = kurtex

