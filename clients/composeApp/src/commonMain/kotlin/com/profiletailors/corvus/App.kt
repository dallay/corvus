package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.theme.CorvusTheme

private const val AgentName = "Corvus Agent"

@Composable
@Preview
fun App() {
  CorvusTheme { ChatWorkspace(state = ChatWorkspaceDefaults.state(modelName = AgentName)) }
}
