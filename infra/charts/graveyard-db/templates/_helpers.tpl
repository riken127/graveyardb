{{/*
Expand the name of the chart.
*/}}
{{- define "graveyard-db.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "graveyard-db.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "graveyard-db.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "graveyard-db.labels" -}}
helm.sh/chart: {{ include "graveyard-db.chart" . }}
{{ include "graveyard-db.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "graveyard-db.selectorLabels" -}}
app.kubernetes.io/name: {{ include "graveyard-db.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Headless service name
*/}}
{{- define "graveyard-db.headlessServiceName" -}}
{{- printf "%s-headless" (include "graveyard-db.fullname" .) -}}
{{- end }}

{{/*
Configmap value for cluster nodes
*/}}
{{- define "graveyard-db.clusterNodes" -}}
{{- $nodes := list -}}
{{- $fullname := include "graveyard-db.fullname" . -}}
{{- $namespace := .Release.Namespace -}}
{{- $serviceName := include "graveyard-db.headlessServiceName" . -}}
{{- $port := int .Values.service.headlessPort -}}
{{- range $i := until (int .Values.replicaCount) -}}
{{- $nodes = append $nodes (printf "%s-%d.%s.%s.svc.cluster.local:%d" $fullname $i $serviceName $namespace $port) -}}
{{- end -}}
{{- join "," $nodes -}}
{{- end }}

{{/*
Auth secret name
*/}}
{{- define "graveyard-db.authSecretName" -}}
{{- default (printf "%s-auth" (include "graveyard-db.fullname" .)) .Values.auth.existingSecret -}}
{{- end }}

{{/*
Auth token value, preserving an existing cluster secret if one exists.
*/}}
{{- define "graveyard-db.authToken" -}}
{{- if .Values.auth.token -}}
{{- .Values.auth.token -}}
{{- else -}}
{{- $secret := lookup "v1" "Secret" .Release.Namespace (include "graveyard-db.authSecretName" .) -}}
{{- if and $secret $secret.data (hasKey $secret.data .Values.auth.secretKey) -}}
{{- index $secret.data .Values.auth.secretKey | b64dec -}}
{{- else -}}
{{- randAlphaNum 48 -}}
{{- end -}}
{{- end -}}
{{- end }}
