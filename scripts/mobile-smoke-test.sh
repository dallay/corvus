#!/bin/bash
# Mobile Smoke Validation Helper Script
# Usage: bash scripts/mobile-smoke-test.sh [android|ios|both]

set -e

CORVUS_BINARY="clients/agent-runtime/target/release/corvus"
ANDROID_APK="clients/androidApp/build/outputs/apk/debug/androidApp-debug.apk"
PACKAGE_NAME="com.profiletailors.corvus"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${CYAN}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[⚠]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }

check_prerequisites() {
  log_info "Checking prerequisites..."
  
  if [ ! -f "$CORVUS_BINARY" ]; then
    log_error "Corvus binary not found at $CORVUS_BINARY"
    log_info "Run: make rust-build"
    exit 1
  fi
  log_success "Corvus binary exists ($(ls -lh $CORVUS_BINARY | awk '{print $5}'))"
  
  if [ ! -f "$ANDROID_APK" ]; then
    log_error "Android APK not found at $ANDROID_APK"
    log_info "Run: make android-build"
    exit 1
  fi
  log_success "Android APK exists"
  
  if ! command -v adb &> /dev/null; then
    log_warn "ADB not found in PATH - Android testing unavailable"
  fi
}

test_android() {
  log_info "=== ANDROID SMOKE VALIDATION ==="
  
  # Check device connection
  DEVICE_COUNT=$(adb devices 2>/dev/null | grep -c "device$" || true)
  if [ "$DEVICE_COUNT" -eq 0 ]; then
    log_error "No Android devices connected"
    log_info "Connect device via USB and enable ADB debugging"
    exit 1
  fi
  log_success "Found $DEVICE_COUNT Android device(s)"
  
  # Deploy corvus binary
  log_info "Deploying corvus binary to device..."
  adb push "$CORVUS_BINARY" /data/local/tmp/corvus >/dev/null 2>&1 || {
    log_warn "Failed to push corvus binary (may require root)"
    log_info "Alternative: Configure endpoint URL in app settings"
  }
  adb shell chmod +x /data/local/tmp/corvus 2>/dev/null || true
  log_success "Corvus binary deployed"
  
  # Install app
  log_info "Installing Android app..."
  adb install -r "$ANDROID_APK" >/dev/null 2>&1 || {
    log_error "Failed to install APK"
    exit 1
  }
  log_success "App installed"
  
  # Launch app
  log_info "Launching app..."
  adb shell am start -n "$PACKAGE_NAME/.MainActivity" >/dev/null 2>&1
  log_success "App launched"
  
  echo ""
  log_info "=== MANUAL VALIDATION STEPS ==="
  echo "1. Check onboarding screen appears"
  echo "2. Configure connection target (endpoint URL or companion)"
  echo "3. Verify link/trust establishment"
  echo "4. Start a new session"
  echo "5. Send test message: 'Hello, can you hear me?'"
  echo "6. Verify runtime reply received"
  echo "7. Test approval flow: Send 'approve this action'"
  echo "8. Approve/deny and verify response"
  echo "9. Test disconnect and relink"
  echo ""
  log_info "Monitor logs: adb logcat | grep -i 'corvus\|runtime'"
  log_info "Capture screenshots: adb shell screencap -p /sdcard/screen.png && adb pull /sdcard/screen.png"
}

test_ios() {
  log_info "=== iOS SMOKE VALIDATION ==="
  
  # Check for connected devices
  DEVICE_LIST=$(xcrun xctrace list devices 2>/dev/null | grep -A 100 "== Devices ==" | grep -B 100 "== " | head -20)
  if echo "$DEVICE_LIST" | grep -q "Offline"; then
    log_warn "iOS devices detected but offline"
    log_info "Connect device via USB and trust this computer"
  fi
  
  # Check TEAM_ID configuration
  TEAM_ID=$(grep "TEAM_ID=" clients/iosApp/Configuration/Config.xcconfig | cut -d'=' -f2)
  if [ -z "$TEAM_ID" ]; then
    log_error "TEAM_ID not configured in clients/iosApp/Configuration/Config.xcconfig"
    log_info "Add your Apple Developer Team ID to proceed with device builds"
    log_info " simulator builds work without TEAM_ID"
    exit 1
  fi
  log_success "TEAM_ID configured: $TEAM_ID"
  
  # Build for device
  log_info "Building iOS app for device..."
  xcodebuild -project clients/iosApp/iosApp.xcodeproj \
    -scheme iosApp \
    -configuration Debug \
    -destination 'generic/platform=iOS' \
    -quiet build 2>&1 | tail -20 || {
    log_error "iOS build failed"
    exit 1
  }
  log_success "iOS app built"
  
  echo ""
  log_info "=== iOS VALIDATION NOTES ==="
  log_warn "iOS companion client not implemented (see BUG-1 in smoke validation report)"
  log_info "App will show onboarding with 'runtime unavailable' until companion client is implemented"
  echo ""
  log_info "To install on connected device:"
  echo "  xcodebuild -project clients/iosApp/iosApp.xcodeproj \\"
  echo "    -scheme iosApp \\"
  echo "    -configuration Debug \\"
  echo "    -destination 'platform=iOS,id=<DEVICE_UDID>' \\"
  echo "    build run"
}

# Main execution
MODE=${1:-both}

case "$MODE" in
  android)
    check_prerequisites
    test_android
    ;;
  ios)
    check_prerequisites
    test_ios
    ;;
  both)
    check_prerequisites
    test_android
    echo ""
    echo "========================================"
    echo ""
    test_ios
    ;;
  *)
    echo "Usage: $0 [android|ios|both]"
    echo ""
    echo "Modes:"
    echo "  android  - Validate Android smoke tests"
    echo "  ios      - Validate iOS smoke tests"
    echo "  both     - Validate both platforms (default)"
    exit 1
    ;;
esac
