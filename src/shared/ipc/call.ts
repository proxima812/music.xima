import { invoke } from '@tauri-apps/api/core'

import { oneOf } from './enums'
import type { IpcError, IpcErrorCode } from './types'

/**
 * Единственное место в приложении, где вызывается `invoke`.
 * Всё остальное ходит через `commands.ts`.
 */

const IPC_ERROR_CODES: readonly IpcErrorCode[] = [
  'NOT_FOUND',
  'INVALID_INPUT',
  'DATABASE',
  'PLAYER',
  'SCAN',
  'IO',
  'UNSUPPORTED_DELETE',
  'INTERNAL',
]

/** Ошибка IPC в виде, пригодном для `throw`: сохраняет stack и `cause`. */
export class IpcCallError extends Error implements IpcError {
  readonly code: IpcErrorCode

  constructor(code: IpcErrorCode, message: string, cause?: unknown) {
    super(message, { cause })
    this.name = 'IpcCallError'
    this.code = code
  }
}

export function isIpcError(value: unknown): value is IpcError {
  if (typeof value !== 'object' || value === null) return false
  const record: Record<string, unknown> = value as Record<string, unknown>
  if (typeof record['message'] !== 'string') return false
  return oneOf(record['code'], IPC_ERROR_CODES) !== undefined
}

/** Приводит что угодно, прилетевшее из Tauri, к контрактному виду `IpcError` (§2). */
export function toIpcError(raw: unknown): IpcCallError {
  if (raw instanceof IpcCallError) return raw

  if (typeof raw === 'object' && raw !== null) {
    const record: Record<string, unknown> = raw as Record<string, unknown>
    const code = oneOf(record['code'], IPC_ERROR_CODES) ?? 'INTERNAL'
    const message = record['message']
    if (typeof message === 'string') return new IpcCallError(code, message, raw)
  }

  if (typeof raw === 'string') return new IpcCallError('INTERNAL', raw, raw)

  return new IpcCallError('INTERNAL', describe(raw), raw)
}

function describe(raw: unknown): string {
  try {
    const json: string | undefined = JSON.stringify(raw)
    return json === undefined ? String(raw) : json
  } catch {
    return String(raw)
  }
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (raw) {
    const error = toIpcError(raw)
    console.error(`[ipc] ${cmd} → ${error.code}: ${error.message}`, { args, raw })
    throw error
  }
}
