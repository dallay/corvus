package com.profiletailors.app

import org.slf4j.Logger
import org.slf4j.LoggerFactory

/**
 * application
 *
 * @author dy
 */
object Application {

  /**
   * main function
   *
   * @param args arguments
   */
  @JvmStatic
  fun main(args: Array<String>) {
    MainModule().run(args)
  }
}

/**
 * main module
 *
 * @author dy
 */
class MainModule {

  companion object {
    private val log: Logger = LoggerFactory.getLogger(this::class.java)
  }

  /**
   * run
   *
   * @param args arguments
   */
  fun run(args: Array<String>) {
    log.info("Using arguments: ${args.joinToString()}")
    log.info("Starting application...")
  }
}
