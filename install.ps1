# INSTALL SCRIPT FOR WINDOWS

# Get latest release
$repo = "YOUR_USER/YOUR_REPO"
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$downloadUrl = $release.assets[0].browser_download_url

# Download to Desktop
$desktopPath = [Environment]::GetFolderPath("Desktop")
$exePath = "$desktopPath\yourapp.exe"

Write-Host "Downloading..."
Invoke-WebRequest -Uri $downloadUrl -OutFile $exePath
Write-Host "Done! Check your Desktop"
