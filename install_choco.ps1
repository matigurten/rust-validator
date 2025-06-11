# Set execution policy to allow script execution
Set-ExecutionPolicy Bypass -Scope Process -Force

# Set TLS to 1.2
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072

# Create the Chocolatey directory if it doesn't exist
$chocoPath = "C:\ProgramData\chocolatey"
if (!(Test-Path $chocoPath)) {
    New-Item -ItemType Directory -Path $chocoPath -Force
}

# Download and install Chocolatey
$installScript = "https://community.chocolatey.org/install.ps1"
$tempFile = [System.IO.Path]::GetTempFileName()
(New-Object System.Net.WebClient).DownloadFile($installScript, $tempFile)

# Run the installation script
& $tempFile

# Clean up
Remove-Item $tempFile

# Add Chocolatey to PATH
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

Write-Host "Chocolatey installation completed. Please restart your terminal to use 'choco' commands." 