package com.profiletailors.spring

import org.assertj.core.api.Assertions.assertThat
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.extension.ExtendWith
import org.springframework.boot.info.BuildProperties
import org.springframework.boot.resttestclient.autoconfigure.AutoConfigureRestTestClient
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.boot.test.system.CapturedOutput
import org.springframework.boot.test.system.OutputCaptureExtension
import org.springframework.boot.webmvc.test.autoconfigure.AutoConfigureMockMvc
import org.springframework.boot.webmvc.test.autoconfigure.MockMvcPrint
import org.springframework.test.context.TestConstructor
import org.springframework.test.web.servlet.assertj.MockMvcTester
import org.springframework.test.web.servlet.client.RestTestClient
import org.springframework.test.web.servlet.client.assertj.RestTestClientResponse

@SpringBootTest
@AutoConfigureMockMvc(print = MockMvcPrint.SYSTEM_ERR, printOnlyOnFailure = false)
@AutoConfigureRestTestClient
@ExtendWith(OutputCaptureExtension::class)
@TestConstructor(autowireMode = TestConstructor.AutowireMode.ALL)
class DemoApplicationTests(
  private val buildProperties: BuildProperties,
  private val mockMvcTester: MockMvcTester,
  private val restTestClient: RestTestClient,
) {

  @Test
  fun `print build properties`() {
    buildProperties.iterator().forEach { println(it.key + ": " + it.value) }
  }

  @Test
  fun `test mock mvc`(output: CapturedOutput) {
    val spec = mockMvcTester.get().uri("/")
    val response = spec.exchange().response.contentAsString
    println("test mock mvc response: $response")
    assertThat(spec).hasStatusOk().hasBodyTextEqualTo("Hello, Spring Boot!")
    assertThat(output).contains("Request URI = /")
  }

  @Test
  fun `test rest client`() {
    val spec = restTestClient.get().uri("/").exchange()
    val response = spec.expectBody().returnResult().responseBodyContent.decodeToString()
    println("test rest client response: $response")
    assertThat(RestTestClientResponse.from(spec))
      .hasStatusOk()
      .bodyText()
      .isEqualTo("Hello, Spring Boot!")
  }
}
