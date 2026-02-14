package com.profiletailors.spring

import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.runApplication
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.RestController

@SpringBootApplication class SpringApp

@Suppress("detekt:SpreadOperator")
fun main(args: Array<String>) {
  runApplication<SpringApp>(*args)
}

interface ExampleApi {

  @GetMapping("/") fun index(): ResponseEntity<String>
}

@RestController
class ExampleController : ExampleApi {

  override fun index(): ResponseEntity<String> {
    return ResponseEntity.ok("Hello, Spring Boot!")
  }
}
