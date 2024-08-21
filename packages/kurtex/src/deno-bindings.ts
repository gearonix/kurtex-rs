import type {
  CollectorRunMode,
  CreateNode,
  KurtexInternals,
  KurtexPublicApi,
  LifetimeHookType,
  ObjectEntry,
  TaskFactory,
  Test,
  TestCallback,
  TestFactory
} from '@/types'

const { core } = Deno
const { ops } = core

function registerTaskImpl(runMode: CollectorRunMode): TaskFactory {
  return (identifier: string, callback: TestCallback | undefined) => {
    kurtexInternals.registerCollectorTask(
      identifier,
      callback && runMode !== 'todo' ? callback : () => {},
      runMode
    )
  }
}

function registerNodeImpl(runMode: CollectorRunMode) {
  return (identifier: string, factory?: TestFactory) => {
    kurtexInternals.registerCollectorNode(
      identifier,
      factory && runMode !== 'todo' ? factory : () => {},
      runMode
    )
  }
}

function registerLifetimeHookImpl(hook: LifetimeHookType) {
  return (callback: TestCallback) => {
    kurtexInternals.registerLifetimeHook(hook, callback)
  }
}

const registerTask = registerTaskImpl('run') as Test
registerTask.only = registerTaskImpl('only')
registerTask.skip = registerTaskImpl('skip')
registerTask.todo = registerTaskImpl('todo')

const registerNode = registerNodeImpl('run') as CreateNode

registerNode.only = registerNodeImpl('only')
registerNode.skip = registerNodeImpl('skip')
registerNode.todo = registerNodeImpl('todo')

const beforeAllHook = registerLifetimeHookImpl('beforeAll')
const afterAllHook = registerLifetimeHookImpl('afterAll')
const beforeEachHook = registerLifetimeHookImpl('beforeEach')
const afterEachHook = registerLifetimeHookImpl('afterEach')

const kurtexInternals = {
  registerCollectorTask(identifier, callback, mode) {
    ops.op_register_collector_task(identifier, callback, mode)
  },
  registerCollectorNode(identifier, factory, mode) {
    ops.op_register_collector_node(identifier, factory, mode)
  },
  registerLifetimeHook(hook, callback) {
    ops.op_register_lifetime_hook(hook, callback)
  }
} satisfies KurtexInternals

const kurtexPublicApi = {
  test: registerTask,
  it: registerTask,
  createNode: registerNode,
  describe: registerNode,
  suite: registerNode,
  beforeAll: beforeAllHook,
  afterAll: afterAllHook,
  beforeEach: beforeEachHook,
  afterEach: afterEachHook
} satisfies KurtexPublicApi

function registerApiGlobally() {
  const publicApi = Object.entries(
    kurtexPublicApi
  ) as ObjectEntry<KurtexPublicApi>[]

  publicApi.forEach(([key, value]) => {
    // @ts-expect-error globalThis is not supported by Deno plugin
    globalThis[key] = value
  })
}

// @ts-expect-error globalThis is not supported by Deno plugin
globalThis._kurtexInternals = kurtexInternals

registerApiGlobally()
