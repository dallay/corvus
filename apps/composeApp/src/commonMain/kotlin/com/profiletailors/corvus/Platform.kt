package com.profiletailors.corvus

interface Platform {
  val name: String
}

expect fun getPlatform(): Platform
