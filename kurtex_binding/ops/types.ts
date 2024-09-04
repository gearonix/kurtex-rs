type Awaitable<T> = T | Promise<T>

export type TestCallback = () => Awaitable<void>
export type TestFactory = () => Awaitable<void>

export interface TaskFactory {
  (identifier: string, fn?: TestCallback): void
}

export interface CreateNodeFactory {
  (identifier: string): void
}

export interface Test extends TaskFactory {
  skip: TaskFactory
  only: TaskFactory
  todo: TaskFactory
}

export interface CreateNode extends CreateNodeFactory {
  skip: CreateNodeFactory
  only: CreateNodeFactory
  todo: CreateNodeFactory
}

export type LifetimeHook = (callback: TestCallback) => void

export type CollectorRunMode = 'run' | 'skip' | 'only' | 'todo'
export type LifetimeHookType =
  | 'beforeAll'
  | 'afterAll'
  | 'beforeEach'
  | 'afterEach'

type RegisterCollectorTask = (
  identifier: string,
  callback: TestCallback,
  mode: CollectorRunMode
) => void

type RegisterCollectorNode = (
  identifier: string,
  factory: TestFactory,
  runMode: CollectorRunMode
) => void

type RegisterLifetimeHook = (
  hook: LifetimeHookType,
  callback: TestCallback
) => void

export interface KurtexInternals {
  registerCollectorTask: RegisterCollectorTask
  registerCollectorNode: RegisterCollectorNode
  registerLifetimeHook: RegisterLifetimeHook
}

export interface KurtexPublicApi {
  test: Test
  it: Test
  createNode: CreateNode
  suite: CreateNode
  describe: Test
  beforeAll: LifetimeHook
  afterAll: LifetimeHook
  beforeEach: LifetimeHook
  afterEach: LifetimeHook
}

export type ObjectEntry<T> = {
  [Key in Extract<keyof T, string>]: [Key, Exclude<T[Key], undefined>]
}[Extract<keyof T, string>]

