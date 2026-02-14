package com.profiletailors.java;

/// example-java application
///
/// @author dy
public class JavaApp {

  /// say hello
  ///
  /// @return Hello, Java!
  public String sayHello() {
    return "Hello, Java!";
  }

  /// main function
  @SuppressWarnings("SystemOut")
  void main() {
    System.out.println(new JavaApp().sayHello());
  }
}
