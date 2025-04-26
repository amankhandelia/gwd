<#
.SYNOPSIS
Blocks or unblocks websites by modifying the Windows hosts file. Requires Admin privileges.

.DESCRIPTION
This script adds entries to the hosts file to block specified websites or removes them to unblock.
Unblocking requires completing a typing challenge to add friction.

.PARAMETER Block
The website domain name to block (e.g., example.com). Do not include 'http://' or 'www.'.

.PARAMETER Unblock
The website domain name to unblock (e.g., example.com). Do not include 'http://' or 'www.'.

.PARAMETER Help
Displays this help message.

.EXAMPLE
.\WebsiteBlocker.ps1 -Block distracting.com
# Adds '0.0.0.0 distracting.com' and '0.0.0.0 www.distracting.com' to the hosts file.

.EXAMPLE
.\WebsiteBlocker.ps1 -Unblock distracting.com
# Prompts with a typing challenge. If successful, removes entries for distracting.com from the hosts file.

.EXAMPLE
.\WebsiteBlocker.ps1 -h
# Displays the help message.

.NOTES
Requires running PowerShell as Administrator.
Blocking/Unblocking might require flushing the DNS cache (`ipconfig /flushdns`) to take immediate effect.
#>
[CmdletBinding(DefaultParameterSetName = 'Help')]
param(
    [Parameter(Mandatory = $true, ParameterSetName = 'Block', HelpMessage = 'Website domain to block (e.g., example.com)')]
    [string]$Block,

    [Parameter(Mandatory = $true, ParameterSetName = 'Unblock', HelpMessage = 'Website domain to unblock (e.g., example.com)')]
    [string]$Unblock,

    [Parameter(Mandatory = $true, ParameterSetName = 'Help')]
    [switch]$Help
)

# --- Configuration ---
$hostsFilePath = "$env:SystemRoot\System32\drivers\etc\hosts"
$redirectIp = "0.0.0.0" # Use 0.0.0.0 as it tends to fail faster than 127.0.0.1
$blockCommentTag = "# Blocked by WebsiteBlocker Script"

# List of words for the unblock challenge
$challengeWords = @(
    "account", "achieve", "adapt", "administration", "admit", "affect", "agency", "agenda", "almost", "already",
    "although", "analysis", "animal", "another", "answer", "appear", "apply", "approach", "approve", "argue",
    "around", "article", "artist", "assume", "attack", "attention", "attorney", "audience", "author", "available",
    "avoid", "beautiful", "because", "become", "before", "begin", "behavior", "behind", "believe", "benefit",
    "better", "between", "beyond", "billion", "budget", "building", "business", "campaign", "cancer", "candidate",
    "capital", "career", "carry", "catch", "cause", "center", "central", "century", "certain", "challenge",
    "chance", "change", "character", "charge", "check", "choice", "choose", "church", "citizen", "civil",
    "claim", "clear", "close", "coach", "collect", "college", "common", "community", "company", "compare",
    "computer", "concern", "condition", "conference", "congress", "consider", "consumer", "contain", "continue", "control",
    "could", "country", "couple", "course", "court", "cover", "create", "culture", "current", "customer",
    "darkness", "daughter", "debate", "decade", "decide", "decision", "defense", "degree", "democrat", "describe",
    "design", "despite", "detail", "determine", "develop", "difference", "difficult", "dinner", "director", "discover",
    "discuss", "disease", "doctor", "dream", "drive", "during", "economic", "economy", "education", "effect",
    "effort", "either", "election", "employee", "energy", "enjoy", "enough", "entire", "environment", "especial",
    "establish", "evening", "event", "every", "evidence", "exactly", "example", "executive", "expect", "experience",
    "explain", "factor", "family", "father", "federal", "feeling", "field", "figure", "final", "financial",
    "finish", "floor", "focus", "follow", "force", "foreign", "forget", "former", "forward", "friend",
    "future", "garden", "general", "generation", "glass", "global", "ground", "growth", "guess", "happen",
    "health", "heart", "history", "hospital", "hotel", "however", "hundred", "husband", "identify", "imagine",
    "impact", "important", "improve", "include", "increase", "indeed", "indicate", "industry", "inform", "instead",
    "interest", "interview", "involve", "issue", "itself", "knowledge", "language", "large", "later", "laugh",
    "lawyer", "leader", "learn", "leave", "legal", "letter", "level", "listen", "little", "local",
    "machine", "magazine", "maintain", "major", "manage", "manager", "market", "marriage", "material", "matter",
    "measure", "media", "medical", "meeting", "member", "memory", "mention", "message", "method", "middle",
    "might", "military", "million", "minute", "mission", "model", "modern", "moment", "money", "month",
    "morning", "mother", "movement", "movie", "music", "myself", "nation", "natural", "nearly", "necessary",
    "network", "never", "north", "notice", "occur", "offer", "office", "officer", "official", "often",
    "operation", "opportunity", "option", "order", "organization", "other", "outside", "owner", "painting", "paper",
    "parent", "partner", "party", "patient", "pattern", "peace", "people", "perform", "period", "person",
    "personal", "phone", "physical", "picture", "piece", "place", "plant", "player", "point", "policy",
    "political", "politics", "popular", "position", "positive", "possible", "power", "practice", "prepare", "present",
    "president", "pressure", "pretty", "prevent", "price", "private", "probably", "problem", "process", "produce",
    "product", "professor", "program", "project", "property", "protect", "prove", "provide", "public", "purpose",
    "quality", "question", "quickly", "quite", "radio", "raise", "range", "rather", "reach", "ready",
    "reality", "reason", "receive", "recent", "recognize", "record", "reduce", "reflect", "region", "relate",
    "remain", "remember", "remove", "report", "represent", "require", "research", "resource", "respond", "response",
    "result", "return", "reveal", "right", "scene", "school", "science", "season", "second", "section",
    "security", "senior", "sense", "series", "serious", "serve", "service", "seven", "several", "shake",
    "share", "shoot", "short", "should", "shoulder", "significant", "similar", "simple", "since", "single",
    "sister", "skill", "small", "smile", "social", "society", "soldier", "somebody", "someone", "something",
    "sometimes", "source", "south", "space", "speak", "special", "specific", "speech", "spend", "sport",
    "spring", "staff", "stage", "stand", "standard", "start", "state", "station", "still", "stock",
    "stop", "store", "story", "strategy", "street", "strong", "structure", "student", "study", "stuff",
    "style", "subject", "success", "suddenly", "suffer", "suggest", "summer", "support", "surface", "system",
    "table", "teach", "teacher", "technology", "television", "thank", "themselves", "theory", "there", "these",
    "thing", "think", "third", "those", "though", "thought", "thousand", "threat", "three", "through",
    "throw", "tight", "today", "together", "tonight", "total", "tough", "toward", "trade", "traditional",
    "training", "travel", "treat", "treatment", "trial", "trouble", "truth", "under", "understand", "until",
    "usually", "value", "various", "victim", "violence", "visit", "voice", "watch", "water", "weapon",
    "weight", "whatever", "where", "whether", "which", "while", "white", "whole", "whose", "window",
    "within", "without", "woman", "wonder", "worker", "world", "worry", "would", "write", "writer",
    "wrong", "yourself"
)

# --- Functions ---

Function Show-Help {
    Write-Host "Website Blocker/Unblocker Script"
    Write-Host "---------------------------------"
    Write-Host "This script modifies the Windows hosts file to block or unblock websites."
    Write-Host "REQUIRES running PowerShell as an ADMINISTRATOR."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\WebsiteBlocker.ps1 -Block <domain.com>"
    Write-Host "      Example: .\WebsiteBlocker.ps1 -Block example.com"
    Write-Host "      (Blocks example.com and www.example.com)"
    Write-Host ""
    Write-Host "  .\WebsiteBlocker.ps1 -Unblock <domain.com>"
    Write-Host "      Example: .\WebsiteBlocker.ps1 -Unblock example.com"
    Write-Host "      (Requires typing challenge, then removes blocking entries for example.com)"
    Write-Host ""
    Write-Host "  .\WebsiteBlocker.ps1 -h"
    Write-Host "      (Displays this help message)"
    Write-Host ""
    Write-Host "Note: Changes might require flushing DNS cache ('ipconfig /flushdns' in cmd/powershell)."
    Write-Host "---------------------------------"
}

Function Test-IsAdmin {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

Function Format-DomainForHosts ([string]$Domain) {
    # Remove http/https and trailing slashes, convert to lowercase
    return $Domain.ToLower().Replace("http://", "").Replace("https://", "").TrimEnd('/')
}

Function Block-Website ([string]$Domain) {
    $cleanDomain = Format-DomainForHosts $Domain
    if (-not $cleanDomain) {
        Write-Error "Invalid domain name provided: '$Domain'"
        return
    }

    $domainWww = "www.$cleanDomain"

    try {
        # Read the entire content first
        $hostsContent = Get-Content $hostsFilePath -ErrorAction Stop
    }
    catch {
        Write-Error "Failed to read hosts file: $($_.Exception.Message). Ensure you are running as Administrator."
        return
    }

    $entry1 = "$redirectIp $cleanDomain $blockCommentTag"
    $entry2 = "$redirectIp $domainWww $blockCommentTag"

    $needsUpdate = $false
    # Use a temporary list to hold lines to add
    $linesToAdd = New-Object System.Collections.Generic.List[string]

    # Check if entries already exist (case-insensitive check, ignoring comments, ensuring full domain match)
    # Match lines starting with the redirect IP, followed by space(s), then the exact domain,
    # then optionally a space and comment, or just the end of the line.
    $exists1 = $hostsContent | Where-Object { $_ -match "^\s*$([regex]::escape($redirectIp))\s+$([regex]::escape($cleanDomain))(\s+|$)(\s*#.*)?$" }
    $exists2 = $hostsContent | Where-Object { $_ -match "^\s*$([regex]::escape($redirectIp))\s+$([regex]::escape($domainWww))(\s+|$)(\s*#.*)?$" }


    if (-not $exists1) {
        $linesToAdd.Add($entry1)
        $needsUpdate = $true
        Write-Host "Adding entry: $entry1" -ForegroundColor Yellow
    }
    else {
        Write-Host "Block entry for $cleanDomain already seems to exist." -ForegroundColor Green
    }

    if (-not $exists2) {
        $linesToAdd.Add($entry2)
        $needsUpdate = $true
        Write-Host "Adding entry: $entry2" -ForegroundColor Yellow
    }
    else {
        Write-Host "Block entry for $domainWww already seems to exist." -ForegroundColor Green
    }

    if ($needsUpdate) {
        try {
            # Prepare the final content by adding new lines to the original content
            $finalContent = [System.Collections.Generic.List[string]]::new() # Create an empty list
            
            # Add existing content line by line to avoid AddRange type issues
            foreach ($line in $hostsContent) {
                $finalContent.Add($line)
            }

            # Add a blank line before new entries if the file doesn't end with one and has content
            if ($finalContent.Count -gt 0 -and $finalContent[-1].Trim() -ne '') {
                $finalContent.Add("")
            }
            # Add the new entries
            $finalContent.AddRange($linesToAdd)

            # Write the entire modified content back using Set-Content with ASCII encoding (safer for hosts file)
            Set-Content -Path $hostsFilePath -Value $finalContent -Encoding Ascii -ErrorAction Stop
            Write-Host "Successfully updated hosts file to block '$cleanDomain' and '$domainWww'." -ForegroundColor Green
            Write-Host "Run 'ipconfig /flushdns' if the block doesn't take effect immediately." -ForegroundColor Cyan
        }
        catch {
            Write-Error "Failed to write to hosts file: $($_.Exception.Message)"
        }
    }
    else {
        Write-Host "'$cleanDomain' and '$domainWww' appear to be already configured for blocking." -ForegroundColor Green
    }
}

Function Unblock-Website ([string]$Domain) {
    $cleanDomain = Format-DomainForHosts $Domain
    if (-not $cleanDomain) {
        Write-Error "Invalid domain name provided: '$Domain'"
        return
    }

    # --- Typing Challenge ---
    $wordCount = Get-Random -Minimum 5 -Maximum 8 # Adjust count as needed
    $challengeSequence = $challengeWords | Get-Random -Count $wordCount | Sort-Object { Get-Random }
    $challengeString = $challengeSequence -join ' '

    Write-Host "--- Unblock Challenge ---" -ForegroundColor Magenta
    Write-Host "To proceed with unblocking '$cleanDomain', please type the following sequence EXACTLY:"
    Write-Host "$challengeString" -ForegroundColor Yellow
    Write-Host "-------------------------" -ForegroundColor Magenta

    $userInput = Read-Host -Prompt "Enter the sequence"

    if ($userInput -ne $challengeString) {
        Write-Error "Incorrect sequence entered. Unblocking cancelled."
        return
    }

    Write-Host "Challenge passed!" -ForegroundColor Green

    # --- Proceed with Unblocking ---
    $domainWww = "www.$cleanDomain"

    try {
        $hostsContent = Get-Content $hostsFilePath -ErrorAction Stop
        $newContent = @()
        $removedCount = 0

        foreach ($line in $hostsContent) {
            # Check if the line contains the blocking entry for the domain or www.domain
            # Match lines starting with the redirect IP, followed by space(s), then the exact domain,
            # then optionally a space and comment, or just the end of the line.
            if ($line -match "^\s*$redirectIp\s+$([regex]::escape($cleanDomain))(\s|$)") {
                Write-Host "Removing line: $line" -ForegroundColor Yellow
                $removedCount++
            }
            elseif ($line -match "^\s*$redirectIp\s+$([regex]::escape($domainWww))(\s|$)") {
                Write-Host "Removing line: $line" -ForegroundColor Yellow
                $removedCount++
            }
            else {
                $newContent += $line
            }
        }

        if ($removedCount -gt 0) {
            try {
                Set-Content -Path $hostsFilePath -Value $newContent -Encoding Default -ErrorAction Stop # Use default encoding, often ANSI for hosts
                Write-Host "Successfully removed $removedCount blocking entries for '$cleanDomain' and '$domainWww' from hosts file." -ForegroundColor Green
                Write-Host "Run 'ipconfig /flushdns' if you still cannot access the site." -ForegroundColor Cyan
            }
            catch {
                Write-Error "Failed to write updated content to hosts file: $($_.Exception.Message)"
            }
        }
        else {
            Write-Host "No active blocking entries found for '$cleanDomain' or '$domainWww' managed by this script." -ForegroundColor Yellow
        }

    }
    catch {
        Write-Error "Failed to read hosts file: $($_.Exception.Message). Ensure you are running as Administrator."
        return
    }
}

# --- Main Script Logic ---

# Check for Admin privileges
if (-not (Test-IsAdmin)) {
    Write-Error "This script requires Administrator privileges to modify the hosts file. Please run PowerShell as Administrator."
    Exit 1
}

# Check if hosts file exists and is writable (basic check)
if (-not (Test-Path $hostsFilePath)) {
    Write-Error "Hosts file not found at '$hostsFilePath'."
    Exit 1
}
# Simple write check (can be improved, but admin check is primary)
try {
    [IO.File]::Open($hostsFilePath, 'Open', 'ReadWrite', 'None').Close()
}
catch {
    Write-Error "Cannot get write access to hosts file at '$hostsFilePath', even running as Admin. Check permissions or if file is locked."
    Exit 1
}


# Parameter Handling
switch ($PSCmdlet.ParameterSetName) {
    'Block' {
        Block-Website -Domain $Block
    }
    'Unblock' {
        Unblock-Website -Domain $Unblock
    }
    'Help' {
        Show-Help
    }
    Default {
        Show-Help # Show help if no valid parameters are provided
    }
}