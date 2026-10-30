// Synthetic net/http downstream for the profile signature E2E (test only).
//
// Uses the LOCAL mycelium-sdk-go checkout through the replace directive in
// go.mod, never a published module. Routes:
//
//   - GET /health: no SDK, always 200.
//   - GET /whoami: SDK adapters/nethttp middleware; verifies the signature
//     when MYC_PROFILE_PUBLIC_KEY_* is set (audience from
//     MYC_PROFILE_SIGNATURE_AUDIENCE).
//   - GET /capture: no SDK; echoes the identity headers it received, only
//     with the private control token.
//   - GET /stats: how many times the /whoami handler actually ran.
//
// Run: go_app <port>
package main

import (
	"crypto/subtle"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"sync/atomic"
	"time"

	mycelium "github.com/LepistaBioinformatics/mycelium-sdk-go"
	"github.com/LepistaBioinformatics/mycelium-sdk-go/adapters/nethttp"
)

const (
	profileHeader   = "x-mycelium-profile"
	signatureHeader = "x-mycelium-profile-signature"
)

var whoamiCalls atomic.Int64

var openapi = map[string]any{
	"openapi": "3.0.3",
	"info":    map[string]any{"title": "PSIG synthetic consumer", "version": "1"},
	"paths": map[string]any{"/whoami": map[string]any{"get": map[string]any{
		"operationId": "psigWhoami",
		"responses":   map[string]any{"200": map[string]any{"description": "Account"}},
	}}},
}

func writeJSON(w http.ResponseWriter, status int, value any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(value)
}

func health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{
		"status":       "ok",
		"runId":        os.Getenv("PSIG_RUN_ID"),
		"verification": os.Getenv("MYC_PROFILE_PUBLIC_KEY_1") != "",
	})
}

func inspect(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]bool{
		"profilePresent":   len(r.Header.Values(profileHeader)) > 0,
		"signaturePresent": len(r.Header.Values(signatureHeader)) > 0,
	})
}

func allowed(r *http.Request) bool {
	expected := []byte(os.Getenv("PSIG_CONTROL_TOKEN"))
	actual := []byte(r.Header.Get("x-psig-control"))
	return len(expected) > 0 && subtle.ConstantTimeCompare(actual, expected) == 1
}

func capture(w http.ResponseWriter, r *http.Request) {
	if !allowed(r) {
		w.WriteHeader(http.StatusForbidden)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{
		"profile":   r.Header.Get(profileHeader),
		"signature": r.Header.Get(signatureHeader),
	})
}

func whoami(w http.ResponseWriter, r *http.Request) {
	whoamiCalls.Add(1)
	profile, ok := nethttp.FromContext(r.Context())
	if !ok {
		writeJSON(w, http.StatusOK, map[string]any{"framework": "nethttp", "accId": nil})
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"framework": "nethttp", "accId": profile.AccID.String()})
}

func stats(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]int64{"whoami": whoamiCalls.Load()})
}

func main() {
	if len(os.Args) != 2 {
		log.Fatal("usage: go_app <port>")
	}
	// Invalid verification settings must stop the service at startup.
	verifier, err := mycelium.ConfigureProfileSignature()
	if err != nil {
		log.Fatalf("profile signature configuration: %v", err)
	}
	protected := nethttp.Middleware(
		nethttp.WithEnvironment("production"),
		nethttp.WithProfileSignatureVerifier(verifier),
	)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", health)
	mux.HandleFunc("GET /openapi.json", func(w http.ResponseWriter, _ *http.Request) { writeJSON(w, http.StatusOK, openapi) })
	mux.HandleFunc("GET /public", inspect)
	mux.HandleFunc("GET /authenticated", inspect)
	mux.HandleFunc("GET /capture", capture)
	mux.HandleFunc("GET /stats", stats)
	mux.Handle("GET /whoami", protected(http.HandlerFunc(whoami)))

	server := &http.Server{Addr: "127.0.0.1:" + os.Args[1], Handler: mux, ReadHeaderTimeout: 10 * time.Second}
	log.Fatal(server.ListenAndServe())
}
