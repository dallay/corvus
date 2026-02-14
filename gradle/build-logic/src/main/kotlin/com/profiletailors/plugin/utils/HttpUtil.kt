package com.profiletailors.plugin.utils

import java.io.File
import java.net.URI
import java.net.URLDecoder
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.time.Duration
import java.util.*
import java.util.concurrent.Executors

/**
 * httpClient build virtual thread
 *
 * Default parameters:
 * - Default timeout 10m
 * - Default executor: virtual thread
 */
val httpClient: HttpClient.Builder =
  HttpClient.newBuilder()
    .executor(Executors.newThreadPerTaskExecutor(Thread.ofVirtual().name("vt-h-c", 0).factory()))
    .connectTimeout(Duration.ofMinutes(10))
    .followRedirects(HttpClient.Redirect.NORMAL)

/** HTTP / URL related utility methods. */
object HttpUtils {

  /** Build URI */
  @Suppress("detekt:ReturnCount")
  fun toURI(base: String, query: Map<String, Any?>? = null): URI {
    if (query.isNullOrEmpty()) return URI.create(base)
    val queryString = encodeMapToQuery(query)
    if (queryString.isEmpty()) return URI.create(base)
    val separator = if (base.contains("?")) "&" else "?"
    return URI.create(base + separator + queryString)
  }

  /**
   * GET request
   *
   * @param uri Request URL
   * @param timeout Response timeout, default 10m
   * @param connectTimeout Connection timeout, default = [timeout]
   * @return body content
   */
  fun get(
    uri: URI,
    timeout: Duration = Duration.ofMinutes(10),
    connectTimeout: Duration = timeout,
  ): String? {
    val request = HttpRequest.newBuilder().uri(uri).GET().timeout(timeout).build()
    val response =
      httpClient
        .connectTimeout(connectTimeout)
        .build()
        .send(request, HttpResponse.BodyHandlers.ofString())
    return response.body()
  }

  /**
   * HEAD request
   *
   * @param uri Request URL
   * @param timeout Response timeout, default 5m
   * @param connectTimeout Connection timeout, default = [timeout]
   * @return Response status code
   */
  fun head(
    uri: URI,
    timeout: Duration = Duration.ofMinutes(5),
    connectTimeout: Duration = timeout,
  ): Boolean {
    return runCatching {
        val request = HttpRequest.newBuilder().uri(uri).HEAD().timeout(timeout).build()
        val response =
          httpClient
            .connectTimeout(connectTimeout)
            .build()
            .send(request, HttpResponse.BodyHandlers.discarding())
        return response.statusCode() == 200
      }
      .getOrDefault(false)
  }

  /**
   * Download file
   *
   * @param uri Complete URL
   * @param timeout Response timeout, default 10m
   * @param connectTimeout Connection timeout, default = [timeout]
   * @return Downloaded file
   */
  fun download(
    uri: URI,
    timeout: Duration = Duration.ofMinutes(10),
    connectTimeout: Duration = timeout,
  ): File {
    val url = uri.toString()
    val fileName =
      url.substringAfterLast("/", "").substringBeforeLast("?").ifBlank {
        UUID.randomUUID().toString().replace("-", "")
      }
    val filePath = Files.createTempDirectory("download_").resolve(fileName)
    httpClient
      .connectTimeout(connectTimeout)
      .build()
      .send(
        HttpRequest.newBuilder().uri(uri).GET().timeout(timeout).build(),
        HttpResponse.BodyHandlers.ofFile(filePath),
      )
    return filePath.toFile()
  }

  /**
   * Encode parameter Map into URL query string.
   *
   * ```
   * ### Encoding rules
   * - `value == null`
   *   - Encoded as `key`
   *   - Represents **presence-only parameter**
   * - `value == ""`
   *   - Encoded as `key=`
   *   - Represents **explicit null value**
   * - `value is String && value.isNotEmpty()`
   *   - Encoded as `key=value`
   * - `value is Iterable / Array`
   *   - Each element applies the above rules independently
   *   - Supports mixed existence of `null / "" / non-null value`
   * ```
   *
   * ### Example
   *
   * ```
   * mapOf("flag" to null)              -> "flag"
   * mapOf("flag" to "")                -> "flag="
   * mapOf("flag" to listOf(null, ""))  -> "flag&flag="
   * mapOf("a" to listOf(1, 2))         -> "a=1&a=2"
   * ```
   *
   * @param map Parameter Map, value allows null / Iterable / Array
   * @return URL query string (excluding `?`)
   */
  fun encodeMapToQuery(map: Map<String, Any?>): String {
    val pairs = buildList {
      map.forEach { (k, v) ->
        when (v) {
          null -> add(k to null)
          is Iterable<*> ->
            v.forEach { elem ->
              when (elem) {
                null -> add(k to null)
                else -> add(k to elem.toString())
              }
            }
          is Array<*> ->
            v.forEach { elem ->
              when (elem) {
                null -> add(k to null)
                else -> add(k to elem.toString())
              }
            }
          else -> add(k to v.toString())
        }
      }
    }

    return pairs.joinToString("&") { (k, v) ->
      val encodedKey = URLEncoder.encode(k, StandardCharsets.UTF_8)
      when (v) {
        null -> encodedKey
        else -> "$encodedKey=${URLEncoder.encode(v, StandardCharsets.UTF_8)}"
      }
    }
  }

  /**
   * Decode URL (or query string) into query parameter Map.
   *
   * ### Decoding rules (three-state semantics)
   *
   * ```
   * - `flag`
   *   - Parsed as `flag -> null`
   *
   * - `flag=`
   *   - Parsed as `flag -> ""`
   *
   * - `flag=value`
   *   - Parsed as `flag -> "value"`
   *
   * - Multiple parameters with the same name
   *   - Collected as `List` in order of appearance
   * ```
   *
   * ### Example
   *
   * ```
   * "?flag"                  -> { flag=[null] }
   * "?flag="                 -> { flag=[""] }
   * "?flag&flag=1"           -> { flag=[null, "1"] }
   * "?a=1&a=2"               -> { a=["1", "2"] }
   * ```
   *
   * @param url Complete URL or string containing only the query part
   * @return Parameter Map, value uses `List<String?>` to represent three-state semantics
   */
  fun decodeQueryToMap(url: String): Map<String, List<String?>> {
    val rawQuery = url.substringAfter('?', "")
    if (rawQuery.isBlank()) return emptyMap()

    return rawQuery
      .split("&")
      .filter { it.isNotBlank() }
      .map { segment ->
        val idx = segment.indexOf('=')
        when {
          idx < 0 -> {
            val key = URLDecoder.decode(segment, StandardCharsets.UTF_8)
            key to null
          }
          idx == segment.length - 1 -> {
            val key = URLDecoder.decode(segment.substring(0, idx), StandardCharsets.UTF_8)
            key to ""
          }
          else -> {
            val key = URLDecoder.decode(segment.substring(0, idx), StandardCharsets.UTF_8)
            val value = URLDecoder.decode(segment.substring(idx + 1), StandardCharsets.UTF_8)
            key to value
          }
        }
      }
      .groupBy({ it.first }, { it.second })
  }
}
