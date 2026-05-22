import { invoke } from '@tauri-apps/api/core'
import type { AppState, McpDefinition } from './types'

export function getAppState() {
  return invoke<AppState>('get_app_state')
}

export function setMcpEnabled(configPath: string, name: string, enabled: boolean) {
  return invoke<AppState>('set_mcp_enabled', { configPath, name, enabled })
}

export function removeMcp(configPath: string, name: string) {
  return invoke<AppState>('remove_mcp', { configPath, name })
}

export function installMcp(configPath: string, name: string, definition: McpDefinition) {
  return invoke<AppState>('install_mcp', { configPath, name, definition })
}

export function copyMcpToProject(sourceConfigPath: string, name: string, projectPath: string) {
  return invoke<string>('copy_mcp_to_project', { sourceConfigPath, name, projectPath })
}

export function installCatalogToProject(projectPath: string, name: string, definition: McpDefinition) {
  return invoke<string>('install_catalog_to_project', { projectPath, name, definition })
}
