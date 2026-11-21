#!/usr/bin/env bash
# Fills an empty FixOps database with demo data through the API, so every rule and audit entry
# applies: staff of each role, clients at each loyalty tier, computers, and orders at every stage.
#
# Usage: FIXOPS_ADMIN_USERNAME=admin FIXOPS_ADMIN_PASSWORD='change-me-now' scripts/seed.sh
# Seeded staff sign in with password `seed-pass-123`.
set -euo pipefail

api=${FIXOPS_API:-http://127.0.0.1:8080/api/v1}
staff_password=seed-pass-123
: "${FIXOPS_ADMIN_USERNAME:?set FIXOPS_ADMIN_USERNAME to an Administrator}"
: "${FIXOPS_ADMIN_PASSWORD:?set FIXOPS_ADMIN_PASSWORD}"

# request METHOD PATH TOKEN [JSON] — prints the response body, fails on an error status.
request() {
    local body
    body=$(curl -sS -X "$1" "$api$2" -H "Authorization: Bearer $3" \
        -H 'Content-Type: application/json' ${4:+-d "$4"} -w '\n%{http_code}')
    local status=${body##*$'\n'}
    body=${body%$'\n'*}
    if [[ $status != 2* ]]; then
        printf '%s %s failed (%s): %s\n' "$1" "$2" "$status" "$body" >&2
        exit 1
    fi
    printf '%s' "$body"
}

sign_in() {
    jq -n --arg u "$1" --arg p "$2" '{username: $u, password: $p}' |
        curl -sSf "$api/sessions" -H 'Content-Type: application/json' -d @- | jq -r .token
}

admin=$(sign_in "$FIXOPS_ADMIN_USERNAME" "$FIXOPS_ADMIN_PASSWORD")
if [[ $(request GET '/clients?limit=1' "$admin") != "[]" ]]; then
    echo "The database already has clients; seed an empty one." >&2
    exit 1
fi

new_staff() {
    request POST /staff "$admin" "$(jq -n --arg u "$1" --arg p "$staff_password" --arg r "$2" \
        '{username: $u, password: $p, role: $r}')" | jq -r .id
}
new_staff olena.reception Receptionist >/dev/null
taras=$(new_staff taras.bench Technician)
ivan=$(new_staff ivan.bench Technician)
recep=$(sign_in olena.reception "$staff_password")
tech_taras=$(sign_in taras.bench "$staff_password")
tech_ivan=$(sign_in ivan.bench "$staff_password")

# new_client NAME PHONE EMAIL TIER — prints the id.
new_client() {
    local id
    id=$(request POST /clients "$recep" "$(jq -n --arg n "$1" --arg p "$2" --arg e "$3" \
        '{name: $n, phone: $p} + (if $e == "" then {} else {email: $e} end)')" | jq -r .id)
    if [[ $4 != Standard ]]; then
        request PUT "/clients/$id/loyalty-tier" "$admin" "{\"loyalty_tier\":\"$4\"}" >/dev/null
    fi
    echo "$id"
}
# new_device CLIENT TYPE SERIAL SYMPTOMS — prints the id.
new_device() {
    request POST /devices "$recep" "$(jq -n --argjson c "$1" --arg t "$2" --arg s "$3" --arg r "$4" \
        '{client_id: $c, device_type: $t, serial_number: $s, reported_symptoms: $r}')" | jq -r .id
}
# new_order TICKET CLIENT DEVICE TECHNICIAN — prints the id.
new_order() {
    request POST /orders "$recep" "$(jq -n --arg t "$1" --argjson c "$2" --argjson d "$3" --argjson a "$4" \
        '{ticket_number: $t, client_id: $c, device_id: $d, assigned_technician_id: $a}')" | jq -r .id
}
put() { request PUT "/orders/$2/$3" "$1" "$4" >/dev/null; }
now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

# diagnose TECH ORDER FINDINGS PARTS LABOR — findings and an estimate, awaiting the client.
diagnose() {
    put "$1" "$2" diagnosis "$(jq -n --arg f "$3" '{diagnostic_findings: $f}')"
    put "$1" "$2" estimate "{\"parts_cents\":$4,\"labor_cents\":$5}"
}
decide() { put "$recep" "$1" decision "{\"decision\":\"$2\",\"decided_at\":\"$(now)\"}"; }
# repair TECH ORDER PARTS LABOR — approved work under way.
repair() {
    put "$1" "$2" repair "{\"status\":\"InProgress\",\"used_parts_cents\":$3,\"completed_labor_cents\":$4}"
}
ready() { put "$1" "$2" repair '{"status":"Ready","repair_checked":true}'; }

bob=$(new_client "Bohdan Melnyk" "+380 50 111 2233" "bohdan@example.com" Standard)
kate=$(new_client "Kateryna Shevchenko" "+380 67 444 5566" "kateryna@example.com" Silver)
studio=$(new_client "Pixel Forge Studio" "+380 44 777 8899" "it@pixelforge.example" Gold)
andriy=$(new_client "Andriy Kovalenko" "+380 93 222 3344" "" Standard)

air=$(new_device "$bob" Laptop C02XM4AIR01 "MacBook Air M4 13-inch: battery drains overnight, gets warm while asleep")
mini=$(new_device "$bob" Desktop H7M4MINI0425 "Mac mini M4: no video output on HDMI")
pro=$(new_device "$kate" Laptop FVFM4PRO1424 "MacBook Pro 14 M4 Pro: keyboard keys stick after a spill")
thinkpad=$(new_device "$kate" Laptop PF4X1C13G26 "Lenovo ThinkPad X1 Carbon Gen 13: cracked screen")
gaming=$(new_device "$studio" Desktop PFS-RTX5080-01 "Custom PC, RTX 5080: shuts down under load")
r760=$(new_device "$studio" Server CN7R760SRV22 "Dell PowerEdge R760: RAID controller reports a degraded array")
dl380=$(new_device "$studio" Server MXQ3DL380G11 "HP ProLiant DL380 Gen11: fan failure alarm")
xps=$(new_device "$andriy" Laptop DXPS13-9350A "Dell XPS 13 9350: will not charge over USB-C")

# Received
new_order FIX-1001 "$bob" "$mini" "$taras" >/dev/null

# Diagnosing, findings saved, estimate awaiting the client's decision
order=$(new_order FIX-1002 "$bob" "$air" "$taras")
diagnose "$tech_taras" "$order" "Swollen battery; sleep drain from a failing battery controller" 12900 6000

# In progress
order=$(new_order FIX-1003 "$kate" "$pro" "$ivan")
diagnose "$tech_ivan" "$order" "Liquid residue under the top case; keyboard assembly needs replacing" 22000 9000
decide "$order" Approved
repair "$tech_ivan" "$order" 22000 4500

# Ready for payment and pickup (Silver: 10% off labor)
order=$(new_order FIX-1004 "$kate" "$thinkpad" "$taras")
diagnose "$tech_taras" "$order" "Cracked 14-inch OLED panel; touch layer intact" 31000 8000
decide "$order" Approved
repair "$tech_taras" "$order" 31000 8000
ready "$tech_taras" "$order"

# Completed (Gold: 20% off labor)
order=$(new_order FIX-1005 "$studio" "$gaming" "$ivan")
diagnose "$tech_ivan" "$order" "PSU trips over-current protection at full GPU load" 18500 5000
decide "$order" Approved
repair "$tech_ivan" "$order" 18500 5000
ready "$tech_ivan" "$order"
total=$(request GET "/orders/$order" "$recep" | jq .settlement_cents)
put "$recep" "$order" payment "{\"amount_cents\":$total,\"paid_at\":\"$(now)\"}"
put "$recep" "$order" pickup "{\"picked_up_at\":\"$(now)\"}"
request POST "/orders/$order/complete" "$recep" >/dev/null

# Waiting on the bench: diagnosis started, no findings yet
order=$(new_order FIX-1006 "$studio" "$r760" "$ivan")
put "$tech_ivan" "$order" diagnosis '{}'

# Received for the second server
new_order FIX-1007 "$studio" "$dl380" "$taras" >/dev/null

# Cancelled after the client declined the estimate: $30 diagnostic fee
order=$(new_order FIX-1008 "$andriy" "$xps" "$taras")
diagnose "$tech_taras" "$order" "USB-C port and charging IC damaged; board repair needed" 9000 7000
decide "$order" Declined
request POST "/orders/$order/cancel" "$recep" >/dev/null

echo "Seeded 3 staff (password $staff_password), 4 clients, 8 computers and 8 orders."
