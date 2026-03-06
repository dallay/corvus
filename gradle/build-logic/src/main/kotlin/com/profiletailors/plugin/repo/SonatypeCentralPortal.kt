package com.profiletailors.plugin.repo

import java.util.*

internal class SonatypeCentralPortal(
  private val mavenCentralUsername: String,
  private val mavenCentralPassword: String,
) {

  val centralPortalBaseUrl = "https://central.sonatype.com"

  fun getAuthorizationHeader(): Pair<String, String> {
    val authToken =
      Base64.getEncoder()
        .encodeToString("$mavenCentralUsername:$mavenCentralPassword".toByteArray())
    return Pair("Authorization", "Bearer $authToken")
  }
}
