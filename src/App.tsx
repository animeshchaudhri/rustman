import "./App.css";
import ApiTester from "./features/apiTester/Dashboard";
import { ThemeProvider } from "./contexts/ThemeContext";
import { UpdateChecker } from "./components/UpdateChecker";

function App() {
  return (
    <ThemeProvider>
      <ApiTester />
      <UpdateChecker />
    </ThemeProvider>
  );
}

export default App;
