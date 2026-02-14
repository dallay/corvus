import com.profiletailors.plugin.injected
import java.net.URI
import java.nio.file.Files
import java.nio.file.StandardCopyOption

// https://github.com/rhysd/actionlint
val actionlintVersion = "1.7.10"
val installDir = "tools/actionlint"

tasks.register("downloadActionlint") {
  group = "other"
  description = "Downloads and installs actionlint"
  val inject = injected

  inputs.property("version", actionlintVersion)
  inputs.property("installDir", installDir)
  outputs.dir(layout.buildDirectory.dir(installDir))

  doLast {
    val downloadVersion = inputs.properties["version"].toString()
    val destDir =
      inject.layout.buildDirectory.dir(inputs.properties["installDir"].toString()).get().asFile
    if (!destDir.exists()) destDir.mkdirs()

    // 1. 检测操作系统
    val osName = System.getProperty("os.name").lowercase()
    val (os, ext) =
      when {
        osName.contains("win") -> "windows" to "zip"
        osName.contains("mac") -> "darwin" to "tar.gz"
        else -> "linux" to "tar.gz" // 默认为 linux
      }

    // 2. 检测架构
    val osArch = System.getProperty("os.arch").lowercase()
    val arch =
      when {
        osArch == "aarch64" || osArch == "arm64" -> "arm64"
        osArch.contains("arm") -> "armv6"
        else -> "amd64" // 覆盖 x86_64, amd64 等
      }

    // 3. 构建下载 URL
    val fileName = "actionlint_${downloadVersion}_${os}_${arch}.${ext}"
    val downloadUrl =
      "https://github.com/rhysd/actionlint/releases/download/v${downloadVersion}/${fileName}"
    val tempFile = destDir.resolve(fileName)

    logger.lifecycle("Detect OS: $os, Arch: $arch")
    logger.lifecycle("Downloading actionlint v$downloadVersion from $downloadUrl ...")

    // 4. 下载文件
    URI.create(downloadUrl).toURL().openStream().use { input ->
      Files.copy(input, tempFile.toPath(), StandardCopyOption.REPLACE_EXISTING)
    }

    // 5. 解压文件
    logger.lifecycle("Extracting to $destDir ...")

    inject.files.copy {
      from(
        if (ext == "zip") inject.archives.zipTree(tempFile)
        else inject.archives.tarTree(inject.archives.gzip(tempFile))
      )
      into(destDir)
    }

    // 6. 设置可执行权限
    if (os != "windows") {
      File(destDir, "actionlint").setExecutable(true)
    }

    logger.lifecycle("actionlint installed successfully at: ${destDir.absolutePath}")

    // 清理临时文件
    tempFile.delete()
  }
}

tasks.register<Exec>("runActionlint") {
  dependsOn("downloadActionlint")
  group = "toolbox"
  description = "Runs actionlint"

  val execName =
    if (System.getProperty("os.name").lowercase().contains("win")) "actionlint.exe"
    else "actionlint"
  val executablePath =
    layout.buildDirectory.dir(installDir).get().file(execName).asFile.absolutePath

  commandLine(executablePath)
  // args("-color")

  isIgnoreExitValue = false
}
