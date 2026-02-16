package com.profiletailors.corvus.ui.onboarding

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

data class OnboardingStep(
  val title: String,
  val description: String,
)

object OnboardingDefaults {
  fun steps(): List<OnboardingStep> =
    listOf(
      OnboardingStep(
        title = "Bienvenido a Corvus",
        description =
          "Configura tu asistente en minutos y habla con tu agente de IA desde el móvil.",
      ),
      OnboardingStep(
        title = "Conecta tu gateway",
        description =
          "Ingresa la URL base, pairing code y credenciales para conectar tu entorno seguro.",
      ),
      OnboardingStep(
        title = "Habla con tu agente",
        description =
          "Envía mensajes, valida respuestas y ajusta la configuración cuando lo necesites.",
      ),
    )
}

@Composable
fun OnboardingScreen(
  step: OnboardingStep,
  isLastStep: Boolean,
  onSkip: () -> Unit,
  onNext: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme

  Column(
    modifier =
      modifier
        .fillMaxSize()
        .background(colors.background)
        .safeContentPadding()
        .padding(horizontal = 20.dp, vertical = 24.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.SpaceBetween,
  ) {
    Spacer(modifier = Modifier.height(24.dp))

    Column(horizontalAlignment = Alignment.CenterHorizontally) {
      Text(
        text = step.title,
        style = MaterialTheme.typography.headlineMedium,
        fontWeight = FontWeight.Bold,
        color = colors.onBackground,
        textAlign = TextAlign.Center,
      )

      Spacer(modifier = Modifier.height(16.dp))

      Text(
        text = step.description,
        style = MaterialTheme.typography.bodyLarge,
        color = colors.onSurfaceVariant,
        textAlign = TextAlign.Center,
      )
    }

    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.SpaceBetween,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      TextButton(onClick = onSkip) { Text(text = "Saltar") }

      Button(onClick = onNext) {
        Text(text = if (isLastStep) "Comenzar" else "Siguiente")
      }
    }
  }
}
