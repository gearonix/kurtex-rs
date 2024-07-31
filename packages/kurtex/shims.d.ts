// eslint-disable-next-line @typescript-eslint/no-namespace
import type {
  CollectorRunMode,
  KurtexInternals,
  KurtexPublicApi,
  TestCallback
} from '@/types'

declare global {
  declare namespace Deno {
    interface DenoCore {
      ops: {
        op_register_collector_task: (
          identifier: string,
          callback: TestCallback,
          mode: CollectorRunMode
        ) => void
      } & Record<string, (...args: any[]) => unknown>
    }

    export const core: DenoCore
  }

  const _kurtexInternals: KurtexInternals
  const kurtexPublicApi: KurtexPublicApi

  const test: KurtexPublicApi['test']
}
