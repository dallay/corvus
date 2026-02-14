package com.profiletailors.plugin.repo

import com.profiletailors.plugin.utils.httpClient
import java.io.File
import java.io.IOException
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.charset.StandardCharsets
import java.time.Duration
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

  /**
   * Publishing By Using the Portal Publisher API
   *
   * see https://central.sonatype.org/publish/publish-portal-api/ see
   * https://central.sonatype.com/api-doc
   */
  @Suppress("unused")
  private fun centralPortalUpload(
    publishingName: String,
    publishingType: String = "USER_MANAGED",
    file: File,
  ): String {
    // check publishingType
    if (publishingType != "USER_MANAGED" && publishingType != "AUTOMATIC") {
      error("publishingType must be USER_MANAGED or AUTOMATIC")
    }
    val url =
      "${centralPortalBaseUrl}/api/v1/publisher/upload" +
        "?publishingType=" +
        publishingType +
        "&name=" +
        URLEncoder.encode(publishingName, StandardCharsets.UTF_8)
    val authorizationHeader = getAuthorizationHeader()

    val postRequest =
      HttpRequest.newBuilder()
        .uri(URI(url))
        .header(authorizationHeader.first, authorizationHeader.second)
        //        .POST(
        //          HttpRequest.MultipartBodyPublishers
        //        )
        .timeout(Duration.ofMinutes(10))
        .build()
    val response = httpClient.build().send(postRequest, HttpResponse.BodyHandlers.ofString())
    if (response.statusCode() != 200) {
      throw IOException("Upload failed: ${response.body()}")
    }
    return response.body()
  }
}
