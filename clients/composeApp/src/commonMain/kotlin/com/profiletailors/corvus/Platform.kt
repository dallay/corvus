package com.profiletailors.corvus

interface Platform {
  val name: String
  val isMobile: Boolean
}

expect fun getPlatform(): Platform
