import com.profiletailors.plugin.spring.SpringBootHelper.getSpringBootVersion
import com.profiletailors.plugin.spring.SpringBootHelper.springBootStarterWebMvc

plugins {
  id("com.profiletailors.module.kotlin")
  id("com.profiletailors.module.spring-boot")
  id("com.profiletailors.tools.spring-openapi")
}

springBootStarterWebMvc(getSpringBootVersion())
