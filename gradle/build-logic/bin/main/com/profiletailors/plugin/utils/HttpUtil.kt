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
 * httpClient 构建虚拟线程
 *
 * 默认参数：
 * - 默认超时 10m
 * - 默认执行器 虚拟线程
 */
val httpClient: HttpClient.Builder =
  HttpClient.newBuilder()
    .executor(Executors.newThreadPerTaskExecutor(Thread.ofVirtual().name("vt-h-c", 0).factory()))
    .connectTimeout(Duration.ofMinutes(10))
    .followRedirects(HttpClient.Redirect.NORMAL)

/** HTTP / URL 相关工具方法。 */
object HttpUtils {

  /** 构建 URI */
  @Suppress("detekt:ReturnCount")
  fun toURI(base: String, query: Map<String, Any?>? = null): URI {
    if (query.isNullOrEmpty()) return URI.create(base)
    val queryString = encodeMapToQuery(query)
    if (queryString.isEmpty()) return URI.create(base)
    val separator = if (base.contains("?")) "&" else "?"
    return URI.create(base + separator + queryString)
  }

  /**
   * GET 请求
   *
   * @param uri 请求地址
   * @param timeout 响应超时 默认 10m
   * @param connectTimeout 连接超时 默认 = [timeout]
   * @return body 内容
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
   * HEAD 请求
   *
   * @param uri 请求地址
   * @param timeout 响应超时 默认 5m
   * @param connectTimeout 连接超时 默认 = [timeout]
   * @return 响应状态码
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
   * 下载文件
   *
   * @param uri 完整 URL
   * @param timeout 响应超时 默认 10m
   * @param connectTimeout 连接超时 默认 = [timeout]
   * @return 下载的文件
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
   * 将参数 Map 编码为 URL 查询字符串。
   *
   * ```
   * ### 编码规则
   * - `value == null`
   *   - 编码为 `key`
   *   - 表示 **presence-only 参数**
   * - `value == ""`
   *   - 编码为 `key=`
   *   - 表示 **显式空值**
   * - `value is String && value.isNotEmpty()`
   *   - 编码为 `key=value`
   * - `value is Iterable / Array`
   *   - 每个元素独立应用上述规则
   *   - 支持 `null / "" / 非空值` 混合存在
   * ```
   *
   * ### 示例
   *
   * ```
   * mapOf("flag" to null)              -> "flag"
   * mapOf("flag" to "")                -> "flag="
   * mapOf("flag" to listOf(null, ""))  -> "flag&flag="
   * mapOf("a" to listOf(1, 2))         -> "a=1&a=2"
   * ```
   *
   * @param map 参数 Map，value 允许为 null / Iterable / Array
   * @return URL 查询字符串（不包含 `?`）
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
   * 将 URL（或查询字符串）解码为查询参数 Map。
   *
   * ### 解码规则（三态语义）
   *
   * ```
   * - `flag`
   *   - 解析为 `flag -> null`
   *
   * - `flag=`
   *   - 解析为 `flag -> ""`
   *
   * - `flag=value`
   *   - 解析为 `flag -> "value"`
   *
   * - 同名参数出现多次
   *   - 按出现顺序收集为 `List`
   * ```
   *
   * ### 示例
   *
   * ```
   * "?flag"                  -> { flag=[null] }
   * "?flag="                 -> { flag=[""] }
   * "?flag&flag=1"           -> { flag=[null, "1"] }
   * "?a=1&a=2"               -> { a=["1", "2"] }
   * ```
   *
   * @param url 完整 URL 或仅包含查询部分的字符串
   * @return 参数 Map，value 使用 `List<String?>` 表示三态语义
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
