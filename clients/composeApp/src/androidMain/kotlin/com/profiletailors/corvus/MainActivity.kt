package com.profiletailors.corvus

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview

class MainActivity : ComponentActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val platform = AndroidPlatform()
    setContent {
      App(platformOverride = platform, initialBridgeSnapshot = defaultBridgeSnapshotFor(platform))
    }
  }
}

@Preview
@Composable
fun AppAndroidPreview() {
  val platform = AndroidPlatform()
  App(platformOverride = platform, initialBridgeSnapshot = defaultBridgeSnapshotFor(platform))
}
