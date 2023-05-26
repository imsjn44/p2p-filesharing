import React, { useContext, useEffect } from 'react';
import './App.css';
import { AppShell, Navbar, Header, Text, MediaQuery, Burger, ActionIcon, Group, Box, Center, Switch } from '@mantine/core';
import { MantineProvider } from '@mantine/core';
import { SunIcon, MoonIcon } from '@modulz/radix-icons';
import { useState } from 'react';
import { createStyles, useMantineTheme } from '@mantine/styles';
import { MemoryRouter, NavLink, Route, Routes } from 'react-router-dom';
import { IconFileReport, IconFileSearch, IconBell } from '@tabler/icons-react';
import search from "../src/images/search.png"
import download from "../src/images/download.png"
import { invoke } from '@tauri-apps/api/tauri';


import RemoteFiles from './RemoteFiles';
import FileTransfer from './FileTransfer';
import { SearhTab } from './SearchTab';
import { MyContext } from './Context';

function App() {

  const {setTransfer } = useContext(MyContext);

  useEffect(()=>{
    const f = async ()=>{
      const x = await invoke("send_active_files");
      setTransfer(x);
    }
    f();
  },[])


  const views = [
    {
      path: 'remotefiles',
      name: 'RemoteFiles',
      component: RemoteFiles,
      image: <img src={search} alt="search" height="55rem" width="55rem" />
      // icon : <IconFileSearch size="2.5rem" stroke={1.5}/>


    },
    {
      path: '/',
      name: 'FileTransfer',
      component: FileTransfer,
      image: <img src={download} alt="download" height="50rem" width="50rem" />
      // icon :<IconFileReport size="2.5rem" stroke={1.5}/>

    }
  ];

  const [opened, setOpened] = useState(false);
  const defaultColorScheme = 'dark';
  const [colorScheme, setColorScheme] = useState(defaultColorScheme);

  const toggleColorScheme = value => {
    const newValue = value || (colorScheme) === 'dark' ? 'light' : 'dark';
    setColorScheme(newValue);
  };

  const useStyles = createStyles((theme) => ({
    navLink: {
      display: 'block',
      width: '100%',
      marginBlockStart: "5.5em",
      marginBlockEnd: "-5rem",
      padding: theme.spacing.xs,
      borderRadius: theme.radius.lg,
      color: colorScheme === 'dark' ? theme.colors.dark[0] : theme.black,
      textDecoration: 'none',

      '&:hover': {
        backgroundColor: colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[1],
      }
    },
    navLinkActive: {

      backgroundColor: colorScheme === 'dark' ? theme.colors.dark[6] : theme.colors.gray[1],
    }
  }));
  const { classes } = useStyles();

  return (
    <MantineProvider theme={{ colorScheme: colorScheme, fontFamily: 'Open Sans, sans serif' }} withGlobalStyles>
      <MemoryRouter>
        <AppShell pading='md' navbarOffsetBreakpoint='sm' fixed

          navbar={

            <Navbar width={{ sm: 150 }} padding='xs' hidden={!opened} hiddenBreakpoint="sm">
              {
                views.map((view, index) =>
                  <NavLink align="left" to={view.path} key={index} onClick={() => setOpened(false)} className={({ isActive }) => classes.navLink + ' ' + (isActive ? classes.navLinkActive : '')}>

                    <Group position='center' style={{ margin: "2rem 0 2rem 0" }}>{view.image}</Group>
                  </NavLink>

                )
              }
            </Navbar>
          }
          header={

            <Header height={70} padding='md'>
              <div style={{ display: 'flex', alignItems: 'center', height: '100%' }}>
                <MediaQuery largerThan='sm' styles={{ display: 'none' }}>
                  <Burger
                    opened={opened}
                    onClick={() => setOpened((o) => !o)}
                    size='sm'
                    color={useMantineTheme().colors.gray[6]}
                    mr='xl'
                  />
                </MediaQuery>

                <Box sx={{ margin: '2rem' }}><img src={require("./images/logo.png")} alt='SwarmNet' height="65px" /></Box>
                <Box sx={{ margin: '3rem' }}>

                  <SearhTab />

                </Box>
                <Group style={{ marginLeft: 'auto' }}>

                  <Switch
                    checked={colorScheme === 'dark'}
                    onChange={() => toggleColorScheme()}
                    size="sm"
                    onLabel={<SunIcon size=".1rem" />}
                    offLabel={<MoonIcon size=".1rem" />}

                  />

                </Group>
              </div>
            </Header>
          }
          styles={theme => ({
            main: { backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[8] : theme.colors.gray[0] },

          })}>
          <Routes>
            {
              views.map((view, index) => <Route key={index} exact={view.exact} path={view.path} element={<view.component />} />)
            }
          </Routes>
        </AppShell>
      </MemoryRouter>
    </MantineProvider>
  );
}

export default App;