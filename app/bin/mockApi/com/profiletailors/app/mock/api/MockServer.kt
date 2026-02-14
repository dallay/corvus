package com.profiletailors.app.mock.api

import com.profiletailors.app.MainModule

/** A mock server. */
class MockServer {

  /**
   * Starts the mock server.
   *
   * @return The list of mock endpoints.
   */
  fun get(): List<String> {
      MainModule()
    return listOf()
  }
}
