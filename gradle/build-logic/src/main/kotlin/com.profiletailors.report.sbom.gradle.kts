plugins {
  java
  id("org.cyclonedx.bom")
}

// Generate a Software Bill of Materials for the software product
tasks.cyclonedxDirectBom {
  includeConfigs.add(configurations.runtimeClasspath.name)
  jsonOutput.set(reporting.baseDirectory.file("sbom/bom.json"))
  xmlOutput.set(reporting.baseDirectory.file("sbom/bom.xml"))
}
